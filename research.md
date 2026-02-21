# Research: MVP Scope for an Elixir-Syntax Fast CLI/TUI Language

> **Track 1 — Definition & Positioning**  
> Date: 2026-02-20 | Author: tidepool

---

## Summary

An Elixir-syntax scripting language targeting fast CLI/TUI apps is an underserved niche:
Elixir developers love the syntax and semantics, but the BEAM VM imposes a 1–3 s cold-start
penalty that makes it a non-starter for CLI scripting. The Babashka model (interpret a
well-chosen language subset, compile to a native binary via GraalVM or a Rust runtime) has proven
the playbook works and produced a ~22 ms startup time. A v0 for this language should nail startup
latency and scripting ergonomics before touching distribution, package management, or TUI
primitives.

---

## 1. Target Users & Jobs-to-be-Done

### Primary users

| Segment | Job |
|---|---|
| **Elixir backend devs** | Replace bash/Python for build scripts, git hooks, and devops automation using syntax they already know |
| **Functional scripting enthusiasts** | Write pipelines, data transforms, and one-shots without boilerplate |
| **Team tooling authors** | Ship a single binary CLI utility to teammates without requiring Elixir/Erlang to be installed |
| **TUI hobbyists (v1+ only)** | Build interactive terminal dashboards or REPL-like tools in a functional style |

### Secondary users (v1+, explicitly deferred from v0)

- Library authors wanting a scripting companion for their Elixir library
- Educators teaching functional programming via a lightweight REPL

### Jobs evidence

- The `/r/elixir` thread "Is Elixir a Good Choice for Building CLI Tools?" (2025) explicitly
  names startup time and absent TUI components as the two blockers.
  [Source](https://www.reddit.com/r/elixir/comments/1j25cyv/is_elixir_a_good_choice_for_building_cli_tools/)
- The Babashka "why" is identical: "The JVM startup time makes it a poor fit for scripting;
  babashka aims to fill this gap." The analogy is direct.
  [Source](https://medium.com/graalvm/babashka-how-graalvm-helped-create-a-fast-starting-scripting-environment-for-clojure-b0fcc38b0746)

---

## 2. v0 Goals & Explicit Non-Goals

### Goals

1. **Sub-100 ms startup on a cold process** (target ≤50 ms; Babashka reference: 22 ms).
2. **Parse and run a useful subset of Elixir syntax** without modification.
   - Pattern matching, pipe operator, modules as namespaces, anonymous functions, guards (basic).
3. **Ship as a single self-contained binary** (no Elixir/Erlang install required).
4. **Standard library adequate for scripting**: file I/O, string manipulation, basic HTTP
   (curl-shelling acceptable in v0), process spawning/shell-out, JSON, regex.
5. **Interop escape hatch**: shell-out (`System.cmd/2`-compatible) so users are never blocked.
6. **Script shebang support** (`#!/usr/bin/env <binary>`).
7. **Inline documentation**: `--help` generation from `@doc` module attributes is a stretch goal
   but signals quality.

### Explicit Non-Goals (v0)

| Non-Goal | Rationale |
|---|---|
| Running on the BEAM | Defeats the startup requirement; contradicts the problem statement |
| Full Elixir compatibility | Scope creep; OTP/GenServer/Supervisor are irrelevant to scripting |
| Package manager / hex.pm integration | Adds enormous surface area; prebuilt stdlib is enough for v0 |
| TUI widgets / bubbletea-equivalent | Blocked on stable runtime first; v1 concern |
| LiveView / Phoenix integration | Out of scope entirely |
| Windows support | Narrow scope for v0; Linux + macOS first |
| Concurrency (Tasks/async) | Hard to implement correctly; deferred to v1 |
| Compiler / AOT for user scripts | Interpretation is sufficient and simpler for v0 |
| IDE tooling / LSP | Nice to have; v1+ |

---

## 3. Minimal Feature Set for First Usable Release

Ordered by priority. A feature not in this list is not in v0.

### Tier 1 — Blockers (nothing ships without these)

- [ ] **Lexer + parser for Elixir surface syntax** (expressions, modules, functions, pattern
      matching, guards, pipe, string interpolation, atoms, tuples, maps, lists)
- [ ] **Tree-walking interpreter** (acceptable for v0; AOT can come later)
- [ ] **Module system** (define and call functions across modules in the same script or via
      `import`/`require`)
- [ ] **Standard library core**:
      `File`, `Path`, `IO`, `String`, `Enum`, `Map`, `List`, `System`, `Regex`, `Jason`-compatible
      JSON
- [ ] **Process shell-out** (`System.cmd/2` and `System.shell/1` equivalents)
- [ ] **Single binary distribution** via static linking or bundling (Rust binary embedding the
      interpreter, or GraalVM native image of a JVM interpreter)
- [ ] **Shebang support**

### Tier 2 — First usable (script ergonomics)

- [ ] **OptionParser** equivalent (flag/arg parsing in stdlib, not third-party)
- [ ] **HTTP client** (shell to `curl` under the hood is acceptable; native client deferred)
- [ ] **Helpful error messages** with file/line context (critical for scripting UX)
- [ ] **`--eval` / `-e` flag** for one-liners: `<binary> -e 'IO.puts("hi")'`
- [ ] **REPL** (read-eval-print loop for exploration; doubles as `iex`-equivalent)

### Tier 3 — Stretch for v0.1

- [ ] **Task runner** (à la `bb tasks` in Babashka — define named tasks in a config file)
- [ ] **`@doc` → `--help` generation**
- [ ] **Environment variable access** (`System.get_env/1`)

### Deliberately excluded from v0

Concurrency primitives, `GenServer`, `Agent`, `ETS`, macros beyond basic `defmacro` parsing,
protocols, structs with behaviours, `Mix` integration, Hex.pm packages.

---

## 4. Performance KPIs

### Startup Latency

> **⚠ Uncertainty**: No published benchmarks exist for the proposed language (it doesn't exist
> yet). All targets are derived from analogous systems; actual numbers must be measured once a
> prototype interpreter exists.

| Tier | Target | Rationale |
|---|---|---|
| **v0 pass** | < 100 ms | Perceptually instant for CLI; matches fast Python (`python -S` ≈ 30 ms) |
| **v0 target** | < 50 ms | Matches the range of interpreted Babashka scripts on modern hardware |
| **Aspirational** | < 25 ms | Babashka's reported 22 ms; achievable with a native binary |
| **Hard fail** | > 300 ms | BEAM escript range; users notice, adoption collapses |

Reference baseline — startup times of analogous tools (from
[bdrung/startup-time](https://github.com/bdrung/startup-time) and Babashka GraalVM blog):

| Tool | Startup |
|---|---|
| Bash | ~7 ms |
| Lua 5.x | ~6 ms |
| Go binary | ~4 ms |
| Babashka (GraalVM native) | **22 ms** |
| Python 3 (`-S`) | ~30 ms |
| Python 3 (full) | ~100–200 ms |
| Node.js | ~80 ms |
| Bun | ~10 ms |
| Deno | ~20–30 ms |
| Elixir escript (BEAM) | **~1,000–3,000 ms** ← the problem |

### Memory (Resident Set Size at idle)

| Tier | Target |
|---|---|
| v0 pass | < 50 MB RSS for a trivial "hello world" script |
| v0 target | < 25 MB RSS |
| Aspirational | < 10 MB RSS (Janet is <1 MB binary; Lua <2 MB) |
| Hard fail | > 100 MB (JVM range without GraalVM) |

> **⚠ Uncertainty**: Memory targets depend heavily on the runtime choice (Rust interpreter vs.
> GraalVM native image). GraalVM native images for Clojure (Babashka) use ~30–60 MB RSS. A
> Rust-based interpreter embedding the language would likely land lower.

### Throughput (Script Execution Speed)

The primary product promise is scripting ergonomics, not compute performance. Still:

| Workload | Target |
|---|---|
| 1M list iterations | Complete in < 2 s (comparable to interpreted Python) |
| 10 MB file line-by-line processing | Complete in < 1 s |
| JSON parse 1 MB payload | Complete in < 500 ms |

> **⚠ Uncertainty**: A tree-walking interpreter will be significantly slower than native Lua or
> Go. Babashka's documentation explicitly states: "If your script takes more than a few seconds
> to run or has lots of loops, Clojure on the JVM may be a better fit." The same caveat applies
> here. Throughput KPIs should be set conservatively and tightened in v1 if a bytecode compiler
> is added.

---

## 5. Adjacent Tools Comparison

| Tool | Lang/Syntax | Runtime mechanism | Startup | Binary size | Stdlib scope | TUI | Pkg ecosystem | Notable weakness |
|---|---|---|---|---|---|---|---|---|
| **Babashka** | Clojure | GraalVM native image of SCI interpreter | **22 ms** | ~80 MB | Large (http, json, csv, sql, …) | via pods | pods + bb.edn | Clojure-only; no Elixir users |
| **Janet** | Lisp (Clojure-ish) | C bytecode VM | ~3 ms | <1 MB | Modest (file, net, math) | None | jpm (small) | Esoteric; low adoption; no pipes |
| **Fennel** | Lisp on Lua | Compiles to Lua, runs in Lua VM | ~6 ms | <1 MB | Lua stdlib | None | Lua luarocks | No native module system; Lua's limits |
| **Nbb** | ClojureScript | Node.js + SCI | ~50–100 ms | 1.2 MB + Node | Node ecosystem | reagent/TUI | npm | Requires Node; cljs not clj |
| **Elixir escript** | Elixir (full) | BEAM VM | **1,000–3,000 ms** | Bundled BEAM | Full Elixir stdlib + OTP | None built-in | Hex.pm (large) | **Startup is the problem this language solves** |
| **Deno** | TypeScript/JS | V8, Rust host | ~20–30 ms | ~80 MB | Large (std, http, fs, …) | None native | jsr/npm | JS/TS only; not functional-first |
| **Bun** | TypeScript/JS | JavaScriptCore, Zig host | ~5–10 ms | ~90 MB | Large (http, sqlite, test, …) | None | npm | JS/TS only; TUI requires npm libs |
| **Rhai** | Rust-like DSL | Rust embedded | <5 ms | Embedded lib | Minimal | None | None | Designed for embedding, not standalone CLIs |

### Sources for comparison table
- Babashka startup: [GraalVM/Medium blog](https://medium.com/graalvm/babashka-how-graalvm-helped-create-a-fast-starting-scripting-environment-for-clojure-b0fcc38b0746)
- Janet overview: [janet-lang.org](https://janet-lang.org/) and [GitHub](https://github.com/janet-lang/janet)
- Fennel: [fennel-lang.org](https://fennel-lang.org/)
- Nbb startup/size: [GitHub babashka/nbb](https://github.com/babashka/nbb)
- BEAM startup time: [ElixirForum thread](https://elixirforum.com/t/how-to-minimise-beam-startup-time/31913)
- Startup times baseline: [bdrung/startup-time](https://github.com/bdrung/startup-time)
- Deno/Bun comparison: [Java Code Geeks](https://www.javacodegeeks.com/2026/02/deno-2-0-vs-node-js-vs-bun-the-complete-javascript-runtime-comparison.html)
- Rhai: [rhai.rs](https://rhai.rs/)

---

## Sources

### Kept

| Source | URL | Why relevant |
|---|---|---|
| Babashka GraalVM blog (Medium/GraalVM) | https://medium.com/graalvm/babashka-how-graalvm-helped-create-a-fast-starting-scripting-environment-for-clojure-b0fcc38b0746 | Primary design narrative + 22 ms startup figure |
| Babashka GitHub | https://github.com/babashka/babashka | Feature list, stdlib decisions, explicit design rationale |
| Babashka Book | https://book.babashka.org/ | Use cases, pod protocol, task runner design |
| Janet lang | https://janet-lang.org/ | Adjacent lightweight Lisp; size/startup reference |
| Fennel lang | https://fennel-lang.org/ | Lisp-on-Lua; zero-overhead compilation model |
| bdrung/startup-time | https://github.com/bdrung/startup-time | Concrete multi-language startup benchmarks |
| Elixir CLI Reddit thread | https://www.reddit.com/r/elixir/comments/1j25cyv/is_elixir_a_good_choice_for_building_cli_tools/ | Primary user evidence for the problem |
| ElixirForum BEAM startup | https://elixirforum.com/t/how-to-minimise-beam-startup-time/31913 | Confirms 1–3 s BEAM penalty; community awareness |
| AppSignal Elixir CLI guide | https://blog.appsignal.com/2022/08/09/write-a-standalone-cli-application-in-elixir.html | Survey of existing Elixir CLI tooling (Burrito, escript) |
| SCI GitHub | https://github.com/babashka/sci | Interpreter design reference for Babashka's core |
| Nbb GitHub | https://github.com/babashka/nbb | Second data point: Clojure subset scripting on Node |

### Dropped

| Source | Why |
|---|---|
| GeeksforGeeks MVP guide | Generic software MVP content; not language-design specific |
| StoriesOnBoard MVP blog | Product management framing; not technical |
| Rye (astral-sh) | Python package manager, not a scripting language |
| Quora Lua performance | Opinion, no sourced data |
| GraalVM native image docs | Relevant for implementation but not for scope research |

---

## Gaps & Uncertainties

### Known unknowns

1. **No published startup-time data for Elixir escript specifically** (only anecdotal "1–3 s" from
   forum posts; no controlled benchmark found). A proper measurement is needed before v0 KPIs
   are locked.

2. **Runtime implementation choice is the single biggest unknnown.** The comparison table above
   assumes a native binary, but the implementation approach isn't decided:
   - **Rust interpreter**: Likely lowest startup and smallest binary; but building a correct
     Elixir subset parser in Rust is significant work.
   - **GraalVM Truffle implementation**: GraalVM Truffle supports adding new languages; an Elixir
     Truffle interpreter could get JIT for free, but GraalVM native image compilation is slow
     (~3–5 minutes) and the resulting binary is large (~80 MB, as with Babashka).
   - **JVM + GraalVM native image**: Babashka's model; well-proven but requires Java toolchain.
   - Each option has very different startup floors. **Recommend prototyping two approaches before
     committing.**

3. **No existing Elixir-syntax scripting runtime was found** (confirming this is greenfield, not
   a fork). No prior art to benchmark against or take library code from.

4. **TUI scope is entirely unresearched** — Ratatui (Rust), bubbletea (Go), and Charm Bracelet's
   ecosystem are adjacent references but none with Elixir syntax. Deferred correctly to v1.

5. **Package/library strategy for v1 is undefined.** Babashka solved this with the pod protocol
   (any language can be a pod via a simple bencode protocol). An equivalent isolation boundary
   for this language needs design work before v1 planning.

### Suggested next steps

- Prototype a minimal Elixir-subset parser in Rust and measure parse+eval startup for a trivial
  script. This gives the first real data point.
- Read the [SCI README](https://github.com/babashka/sci) and [Babashka pods spec](https://github.com/babashka/pods) for implementation patterns.
- Run `time elixir -e 'IO.puts("hi")'` and `time mix run --no-halt` on target hardware to get
  concrete BEAM baseline numbers to put in the KPI table.
- Define the pod/plugin story (how do users extend stdlib?) before locking v0 non-goals.
