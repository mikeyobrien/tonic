Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/03-runtime-performance.md` — ~23 KB, ~400 lines.

**What's in it:**

- **4 approaches** broken down with concrete numbers: tree-walking interpreter, bytecode VM, JIT, transpile-to-host
- **Babashka/Sci vs Clojure/JVM anchor table** — startup, memory, binary, sweet-spot, FFI compared side by side
- **Startup/memory reference table** covering 10 runtimes (Babashka, Lua, Janet, Python, Node, Deno, GraalVM native, etc.)
- **FFI and process model options** — libffi/dlopen, subprocess, native modules, async/fibers — with CLI/TUI fit assessment
- **Binary packaging options** — native AOT, fat archive, source+interpreter, embedded library — with size benchmarks
- **v0 recommendation**: tree-walking interpreter in Rust (~3K LOC, < 20 ms startup, < 20 MB RSS) — focus on language feel, not speed
- **v1 recommendation**: register/stack bytecode VM in Rust, single-pass compiler, still one static binary — targeting Lua/CPython throughput range
- **Escape hatches table** — signals for when to reconsider each approach (JIT, transpile-to-JS, JVM target)
- **Gaps section** — notably: TUI event loop integration with scripting runtime, and that GraalVM path is only relevant if you're JVM-hosted (irrelevant for a Rust implementation)