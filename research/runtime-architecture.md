# Research: Runtime Architecture for a New Scripting/CLI/TUI Language

## Summary

For a language targeting scripting, CLI, and TUI workflows the dominant tradeoff
is **startup latency + memory versus peak throughput**. A bytecode VM compiled to a
static native binary (the Janet model) hits every practical sweet spot for v0: <1 ms
cold start, 2–10 MB RSS, clean embedding API, and a path to JIT later. Babashka/SCI
(the GraalVM native-image-wrapped tree-walker) proves that fast startup is achievable
without a JIT, but at the cost of throughput headroom; the approach works because it
re-uses a complete existing language. A brand-new language should own its runtime.

---

## Anchor Points

### Clojure / JVM

- **Model**: source → JVM bytecode → HotSpot JIT  
- **Startup**: 1–7 s (JVM bootstrap + classloading)  
- **RSS idle**: 150–300 MB  
- **Throughput**: near-C after warm-up (HotSpot profile-guided JIT)  
- **Packaging**: fat JAR (requires installed JVM), or GraalVM native-image for standalone binary  
- **Lesson**: excellent peak performance but useless for short-lived scripts or shells

### Babashka / SCI

- **Model**: tree-walking interpreter (SCI) written in Clojure, compiled to native
  binary via GraalVM native-image. SCI is *not* a Truffle interpreter, so user code
  receives **no JIT optimization**; the native binary just means the *host interpreter*
  doesn't need JVM warm-up.
- **Startup**: ~22 ms (native) vs 7+ s (clojure -M) for the same script  
- **RSS idle**: ~40–60 MB (GraalVM native has a fixed base footprint)  
- **Throughput**: Python / Ruby tier — fine for glue scripts, painful for tight loops  
- **Packaging**: single self-contained native binary (~60–90 MB), cross-compiled per target  
- **Lesson**: GraalVM native-image is a force multiplier for an *existing* language but
  imposes a large binary baseline, a complex closed-world build, and a reflection
  whitelist burden. Inventing a new language on top of it trades one set of constraints
  for another.

Sources:
- [babashka/babashka](https://github.com/babashka/babashka)
- [Babashka: How GraalVM Helped — Medium/GraalVM](https://medium.com/graalvm/babashka-how-graalvm-helped-create-a-fast-starting-scripting-environment-for-clojure-b0fcc38b0746)
- [babashka/sci](https://github.com/babashka/sci)

---

## Findings

### 1 · Tree-walking Interpreter

Walk the AST at eval time; no compilation phase.

| Metric | Range |
|---|---|
| Startup | As low as <5 ms if host binary is native |
| RSS idle | 5–30 MB |
| Throughput | 50–200× slower than native for hot loops |
| Impl cost | Lowest |

**How it works in practice (SCI)**: SCI is a pure Clojure tree-walker compiled into
babashka via GraalVM native-image. The native binary itself starts in ~22 ms. User
scripts are still interpreted at runtime with no further optimization — hot loops are
slow, but for glue / orchestration scripts this is acceptable.

**Strengths**:
- Fastest path to a working language
- Easiest to make correct (no IR to keep in sync)
- Trivially embeddable (just link the source)

**Weaknesses**:
- Throughput ceiling: loops, number crunching, string processing all suffer
- No bytecode serialization — scripts must be re-parsed on every run
- Difficult to add an optimizer later (AST transformations are not enough)

**Verdict**: Good for a language with purely declarative semantics (config DSL, query
language). Too slow for a general scripting language people will trust for real tasks.

---

### 2 · Bytecode VM

Compile source → compact bytecode; execute in a tight dispatch loop.

| Metric | Range |
|---|---|
| Startup | <1 ms (Janet, Lua 5.x) |
| RSS idle | 2–10 MB |
| Throughput | 5–30× faster than tree-walking; ~2–10× slower than native |
| Impl cost | Medium |

**Reference implementations**:

**Janet** ([janet-lang.org](https://janet-lang.org)):
- Total runtime including core library: <1 MB binary
- Register-based VM; bytecode can be cached to disk (`.jimage`)
- Clean C embedding API; scripts compile to bytecode at load time
- Built-in PEG parser, REPL, assembler — all in under 1 MB
- FFI module for calling C libraries without writing glue code
- Single-file C source, trivially vendored into a Rust/Zig host via cc crate

**Lua 5.x** — register-based VM, 0.63 ms startup, ~1–2 MB RSS, legendary for
embedding (nginx, Redis, Neovim, game engines). The VM design is the reference for
"simple and fast."

**QuickJS** ([bellard.org/quickjs](https://bellard.org/quickjs)):
- Embeddable JS engine: 367 KB x86 code for hello-world, <1 ms startup
- Full ES2023 support; `qjsc` compiles JS → C embedding stubs for static linking
- Useful if the host language wants JS semantics

**Key insight from benchmarks**: The Lua 5.x register VM runs `fib(35)` in ~3.7 s;
LuaJIT (bytecode-only, no JIT) runs the same in ~0.8 s; LuaJIT (JIT enabled) ~0.8 s
also (trivially JIT-able loop). The 5× difference comes from smarter register
allocation and trace caching, not from JIT itself in this case.
[Lua Benchmarks — github.com/Jipok](https://github.com/Jipok/Lua-Benchmarks)

**Strengths**:
- Startup and memory match or beat any other approach
- Bytecode is serializable → scripts can be pre-compiled to `.bc` files
- Embedding via C API is mature and well-understood
- Clear upgrade path: add a JIT later over the same bytecode format

**Weaknesses**:
- More implementation work than tree-walking (need a compiler pass)
- Stack or register VM design is a non-trivial decision to reverse

**Verdict**: **Best v0 choice for a new language**. Matches real user expectations
(sub-millisecond shell replacement), keeps options open for v1 JIT.

---

### 3 · JIT Compilation

Observe hot code paths at runtime; emit native code for them.

| Metric | Range |
|---|---|
| Startup | 0.3–5 ms (but cold first runs are slower until traces compile) |
| Warm throughput | Near-C for numeric/loop code |
| RSS idle | 15–100 MB (JIT code cache + metadata) |
| Warmup penalty | 100 ms – 1 s before peak performance |
| Impl cost | Very high |

**LuaJIT**: The canonical scripting JIT. Traces hot loops, emits x86/ARM machine code.
Near-C throughput for numeric code. But it's 2009-era x64 assembly and essentially
unmaintained (Mike Pall stopped active development). Embedding LuaJIT is reasonable;
*building* a JIT like LuaJIT is a multi-year effort.

**GraalVM / Truffle**: Implement your language as a Truffle interpreter (Java AST
nodes with `@Specialization` annotations) and get a JIT for free via Graal's partial
evaluation. Used by TruffleRuby, FastR, GraalPy. But: you're now in the JVM ecosystem,
GraalVM native-image for Truffle languages is complex, and startup still lags behind
native VMs by 50–200 ms even with native-image.

**Cranelift** ([cranelift.dev](https://cranelift.dev)): A pure-Rust code generation
library. Used by Wasmtime for JIT. Can be embedded in a Rust language runtime. Compiles
faster than LLVM, produces 20–30% slower code than LLVM. Good for a v1 JIT backend on
a language whose VM is already in Rust.

**LLVM**: Gold standard for AOT and JIT code quality. Complex to embed (~50 MB shared
library), but gives access to decades of optimizer research. Practical approach: compile
to LLVM IR for an ahead-of-time "fast path" (ship `.bc` precompiled scripts), defer
JIT to v2+.

**Verdict**: Do not implement a JIT for v0. Add Cranelift-backed AOT as a v1 option;
a proper JIT only after bytecode VM is proven stable.

---

### 4 · Transpile-to-Host

Emit valid code in another language and run via that language's runtime.

| Metric | Range |
|---|---|
| Startup | Inherited from host (Lua: <1 ms; Node: 40–80 ms; Python: 25 ms) |
| Throughput | Inherited from host (LuaJIT: near-C; Node V8: fast) |
| RSS idle | Inherited from host |
| Impl cost | Low–medium (frontend only; no VM) |

**Fennel → Lua** ([fennel-lang.org](https://fennel-lang.org)):
- Fennel is a Lisp that compiles to readable Lua, inheriting LuaJIT FFI and the full
  Lua ecosystem for free
- Startup: indistinguishable from Lua itself (<1 ms)
- Distribution: single-file Lua script or compiled binary via luajit/lua + squish
- Full Lua ↔ Fennel interop with no bridge overhead
- Limitation: your semantics must map cleanly to Lua semantics

**Your language → TypeScript/JavaScript**:
- Access V8/Bun/Deno JIT; Node startup 40–80 ms (improving with Bun: ~5 ms)
- Rich package ecosystem
- Debugging is hard (source maps required for usable stack traces)

**Your language → C or Zig**:
- Maximum portability, maximum performance
- Ship one static binary per target
- Complex to emit correct C for all edge cases (GC, continuations, etc.)

**Your language → WASM**:
- Runs in any WASM host, sandboxed
- Growing CLI tooling via WASI
- Startup penalty in WASM runtime (~2–10 ms per module load)

**Verdict**: Transpile-to-Lua is excellent if your language semantics are Lua-adjacent
(or if you're building on Lua intentionally). For a genuinely novel language, owning
the runtime gives more control over error messages, debuggability, and the v1 JIT path.

---

## Startup / Memory Tradeoff Summary

| Runtime | Cold Start | RSS Idle | Peak Throughput | Binary Size |
|---|---|---|---|---|
| Clojure / JVM | 1–7 s | 150–300 MB | Near-C (JIT) | JAR + JVM |
| Babashka / SCI (native) | ~22 ms | 40–60 MB | Python-tier | ~70 MB |
| Python 3.x | 25–50 ms | 15–30 MB | Slow | system dep |
| Node.js / V8 | 40–80 ms | 30–60 MB | Fast (JIT) | system dep |
| Ruby | 60–120 ms | 25–50 MB | Slow | system dep |
| **Janet bytecode VM** | **<1 ms** | **2–5 MB** | 5–10× faster than Python | **<1 MB** |
| **Lua 5.x VM** | **0.63 ms** | **1–2 MB** | Similar to Janet | **~200 KB** |
| LuaJIT (JIT on) | <1 ms | 8–20 MB | Near-C for numbers | ~500 KB |
| QuickJS | <1 ms | 2–5 MB | Moderate (no JIT) | <1 MB |
| Wren (bytecode) | <1 ms | 2–5 MB | Moderate | ~300 KB |

Sources: [bdrung/startup-time](https://github.com/bdrung/startup-time),
[Jipok/Lua-Benchmarks](https://github.com/Jipok/Lua-Benchmarks),
[drujensen/fib](https://github.com/drujensen/fib),
[Wren performance](https://wren.io/performance.html)

---

## Embedding Approach

**Tree-walking interpreter**: embed by linking the source (if pure-host), or via a
C API if implemented in C. API surface is wide and tightly coupled to interpreter state.

**Bytecode VM** (recommended):
- Janet: `janet_init() / janet_core_env() / janet_dobytes()` — 5 calls to embed a
  full runtime. Custom C functions registered with `janet_wrap_cfunction`.
- Lua: `luaL_newstate() / luaL_openlibs() / luaL_dostring()` — similar pattern.
- Your own VM: define a `VM` struct, a `vm_eval(vm, bytecode)` entry point. Let the
  embedder control memory allocation via callbacks (like Lua's `lua_Alloc`).
- Bytecode can be pre-compiled offline and loaded as a byte array — no source code
  needed in embedded deployments.

**JIT**: generally not cleanly embeddable unless the JIT is a library (Cranelift,
LLVM via C API). LuaJIT's embedding API is Lua-compatible, so existing Lua embeddings
upgrade painlessly.

**Transpile-to-host**: embedding *is* embedding the host runtime. Inherits all of the
host's embedding story (Lua: excellent; Node: heavy; Python: decent but GIL issues).

---

## FFI / Process Model Options

### Subprocess / Shell-out

- Zero implementation cost
- Per-call overhead: ~5–50 ms (fork + exec + pipe)
- Good for CLI orchestration (Babashka's primary use case), bad for hot paths
- No shared memory; data passed via stdin/stdout/env

### C FFI (in-process)

- **LuaJIT FFI**: parse C header declarations at runtime, call any `.so` symbol
  directly with no glue code. Near-zero overhead per call. Remarkable engineering.
- **Janet FFI module**: similar — declare function signatures in Janet, load via
  `(ffi/native "libfoo.so")`.
- **libffi-backed**: language runtime uses libffi to construct call frames at runtime.
  ~50 ns overhead per call. Works on all platforms, no JIT required.
- **Hand-rolled C extensions**: compile-time glue (like CPython `.so` modules).
  Zero call overhead, but requires matching ABI versions.

### IPC / Sockets

- Good for long-running companion processes (LSP server, watch daemon)
- Protocol: msgpack, JSON-RPC, or a custom framing
- Not suitable for per-operation FFI

### WASM / Plugin Model

- Sandbox untrusted extensions
- WASI provides filesystem, clock, etc.
- Tools like `extism` or `wasmtime`-embedded runtimes make this practical
- 2–10 ms per module instantiation; subsequent calls ~10–100 ns overhead

**Recommendation for v0**: start with subprocess shell-out (zero cost, already works)
and a thin C FFI via libffi. For v1 add a proper in-process module system with
`.so`/`.dylib` loading.

---

## Packaging Options

### Single Native Binary (Static Link)

The gold standard for CLI tools. Zero runtime dependencies.

- **Rust host runtime**: use `cc` crate to compile Janet/Lua C source and link
  statically. Final binary: 2–8 MB. Works with musl for fully static Linux builds.
- **Zig host runtime**: same approach with even smaller binaries and trivial
  cross-compilation.
- **GraalVM native-image**: works but produces ~50–100 MB binaries, requires
  whitelist-driven reflection configuration, long build times (minutes). Best
  justified when re-using the entire JVM ecosystem (Babashka).

### Bytecode Archive

Pre-compile user scripts to `.bc` / `.jimage` files. Distribute as:
- Single binary that embeds the `.bc` as a resource (`include_bytes!` in Rust)
- Or a two-file distribution: `runtime` + `app.bc`

### AppImage / Bundle

For Linux desktop TUI apps — bundle binary + shared libs into AppImage. 5–50 MB.

### Nix / Homebrew / apt

Package manager distribution. Good for developer tooling; requires per-distro
maintenance.

### Container

For server contexts — not relevant for CLI/TUI latency requirements.

**Recommendation**: ship a static binary with embedded standard library bytecode.
Distribution becomes `cp tonic /usr/local/bin` and nothing else.

---

## Recommendation: v0 and v1 Path

### v0 — Bytecode VM, Static Binary

Build a stack-based or register-based bytecode VM in **Rust** (or Zig if you want
smaller binaries and simpler cross-compile).

**Why stack-based for v0**: simpler to implement and verify. Register-based is 20–30%
faster but requires a register allocator; add this in v1.

Milestones:
1. Lexer + recursive descent parser → AST
2. AST → bytecode compiler (simple single-pass)
3. Stack-based dispatch loop (switch/threaded)
4. Core stdlib: strings, lists, maps, I/O, process/subprocess
5. REPL (read → compile → eval loop over the VM)
6. Compile to single static binary with stdlib embedded as bytecode

**Target metrics at v0 ship**:
- Cold start: <2 ms
- RSS idle: <10 MB
- Binary size: <5 MB (static, no deps)
- Throughput: Lua 5.x tier (good enough for any script a human would write)

**FFI at v0**: subprocess shell-out + a thin `ffi` module via libffi (500 LOC in C
wrapped by your VM's foreign-call ABI). This covers >95% of real-world integration
needs.

**Embedding at v0**: define a `vm_t` struct and a `<lang>_eval_bytes(vm_t*, uint8_t*,
size_t)` C function. That's enough to embed in another application.

### v1 — Performance, Packaging, Ecosystem

1. **Switch to register VM** (or add a bytecode optimization pass)
2. **Add Cranelift-backed AOT compilation**:
   - Emit CLIR (Cranelift IR) from your bytecode
   - Produce native `.so` or statically-linkable object files for hot modules
   - User-visible as `tonic compile foo.tn` → `foo.so` loaded at startup
3. **Proper C extension module system** (`.so` modules with a stable ABI)
4. **Source maps + debugger protocol** (DAP)
5. **Language server** (LSP) as a long-running companion process
6. **Package manager** with lockfile (even just a directory-based one)

**Do not add a JIT for v1** unless profiling shows sustained compute loops are the
bottleneck. For scripting/CLI/TUI, startup time and ergonomics matter more than
throughput. A Cranelift-based AOT option covers the "I need this to be fast" case.

### What to Borrow from Babashka / SCI

- SCI proves that **startup time is the user-visible metric** for scripts; peak
  throughput rarely matters
- The "batteries-included" design (HTTP client, file utilities, JSON, etc.) bundled
  into the binary matters more than raw speed
- Babashka's nbb variant (SCI on Node.js) shows that switching host runtimes is
  possible when the interpreter layer is clean

### What to Avoid from Babashka / SCI

- GraalVM native-image build complexity for a *new* language — not worth it
- Tree-walking as the sole execution strategy — bytecode is not hard and the payoff
  in user trust (loops don't feel broken) is large

---

## Sources

**Kept**:
- [babashka/babashka — GitHub](https://github.com/babashka/babashka) — primary source on SCI + GraalVM model
- [Babashka: How GraalVM Helped — Medium](https://medium.com/graalvm/babashka-how-graalvm-helped-create-a-fast-starting-scripting-environment-for-clojure-b0fcc38b0746) — startup numbers (22 ms vs 7 s), design rationale
- [babashka/sci — GitHub](https://github.com/babashka/sci) — SCI interpreter design
- [janet-lang/janet — GitHub](https://github.com/janet-lang/janet) — bytecode VM reference, <1 MB claim
- [janet-lang.org](https://janet-lang.org) — embedding API, PEG, REPL
- [QuickJS — bellard.org](https://bellard.org/quickjs/) — small embeddable VM reference
- [bdrung/startup-time — GitHub](https://github.com/bdrung/startup-time) — Lua 0.63 ms, Python 25 ms startup data
- [Jipok/Lua-Benchmarks — GitHub](https://github.com/Jipok/Lua-Benchmarks) — Lua vs LuaJIT throughput
- [Wren performance](https://wren.io/performance.html) — cross-language benchmark methodology
- [cranelift.dev](https://cranelift.dev) — Cranelift as embeddable JIT backend
- [cranelift-jit-demo — GitHub](https://github.com/bytecodealliance/cranelift-jit-demo) — practical Cranelift embedding example
- [drujensen/fib — GitHub](https://github.com/drujensen/fib) — janet, lua, luajit, python fib benchmarks
- [Fennel lang](https://fennel-lang.org) — transpile-to-host reference
- [GraalVM native-image](https://www.graalvm.org/latest/reference-manual/native-image/) — AOT Java compilation

**Dropped**:
- SEO articles about "best scripting language 2024" — no benchmarks, no design detail
- tree-sitter WASM packaging results — off-topic (parser, not runtime)
- Reddit threads without data — anecdote only

---

## Gaps

1. **Concrete Janet startup/RSS numbers**: Janet's "< 1 ms" claim is widely cited but
   the exact measured startup under `hyperfine` on modern hardware (with cold disk
   cache) isn't published in an authoritative benchmark. Suggest running
   `hyperfine 'janet -e "(print 1)"'` locally as a calibration point.

2. **Cranelift vs LLVM quality gap for scripting workloads**: the existing comparison
   is Wasmtime-focused (numeric/WASM ops). The 20–30% gap might be larger for dynamic
   dispatch-heavy scripting patterns. Worth a micro-benchmark before committing.

3. **Register vs stack VM throughput delta at scripting workloads**: the 20–30% figure
   comes from Java/Python literature. For CLI-scale scripts (typically <10 ms wall
   time) the difference may be negligible. Worth evaluating whether register VM
   complexity is justified for v0.

4. **GraalVM Truffle + native-image for a custom Truffle language**: it's possible to
   build a new language as a Truffle interpreter and get JIT + native-image. This path
   wasn't fully explored. It could be viable if JVM ecosystem access is desirable.
   Estimated ramp-up: 3–6 months additional complexity.
