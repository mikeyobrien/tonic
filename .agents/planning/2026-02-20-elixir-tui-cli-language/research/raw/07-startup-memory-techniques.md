Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/07-startup-memory-techniques.md` (~26 KB).

---

**What's in the document:**

**9 core findings** with inline citations:
1. **Parser/AST costs** — why grammar complexity and eager AST construction are the first bottleneck; V8's pre-parsing as the mitigation pattern
2. **Bytecode caching** — Bun (`.jsc` + JSC version hash), Deno (V8 snapshot + code cache warmup in 2024), Node (js2c + embedded code cache), Python (`__pycache__`) — all four patterns with their invalidation strategies
3. **Symbol/string interning** — O(1) pointer equality, 5–15 % heap savings at startup, Lua's global `lstring.c` intern table, Python's `sys.intern()`, and the "never-intern untrusted input" guard
4. **Arena allocation** — bump pointer mechanics, 10–50× throughput vs. malloc, Luau's per-function AST arenas; mimalloc vs. jemalloc RSS comparison (<4 MB vs. ~9 MB)
5. **GC strategy choices** — comparison table (STW / incremental / generational / concurrent), Lua 5.4's dual incremental+generational mode, Luau's PID-controller pacer, why GC tuning is *low-leverage* for startup specifically
6. **Cold-start path trimming** — Node's 133K page faults/s, 60 ms / 230M instructions baseline, V8/JSC engine init cost, `readahead` / `posix_fadvise` hints
7. **Lazy module loading** — 60 % NestJS reduction case study, Python `LazyLoader`, Node `require()` cache behavior, stdlib core/extension split recommendation
8. **Static linking** — comparison table (startup, RSS, portability, security), why single-binary CLI tools should always static-link; Bun/Deno/Babashka all do this
9. **Measurements table** — Lua (~2 ms), Babashka (~15 ms), Bun (~10 ms), Deno (~30 ms), Node (~80 ms), JVM Clojure (~2000 ms) with sources

**Actionable checklist** (design-time + operational) and **benchmark methodology** with concrete `hyperfine` + `/usr/bin/time -v` + `strace` commands, plus a confounders section (CPU scaling, ASLR, filesystem cache, JIT tiers).

**5 documented gaps** — notably the BEAM/Elixir cold-start story (directly relevant to the parent planning context) is flagged as unresearched.