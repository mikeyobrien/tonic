Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/06-runtime-semantics-gap.md`.

Here's what the doc covers and why each call was made:

---

**Structure of the research (7 findings + decision table):**

1. **BEAM baseline** — Established the adversary: 400–1000 ms cold start from VM/OTP init vs. 5–20 ms native. This is the target to beat, not optimize around.

2. **Immutability defaults** — Recommended immutable-by-default with a `mut` escape hatch. Startup impact is negligible (compile-time concern), memory overhead is O(log n) via structural sharing — acceptable for CLI-sized data. Cited three primary sources.

3. **Pattern matching** — Maranget's decision-tree algorithm, enforced exhaustiveness. Zero runtime overhead (all work done at compile time). Three primary academic sources cited. Key: exhaustiveness errors at build time = no runtime unmatched-pattern panics.

4. **Exceptions vs. Result types** — `Result(T, E)` wins for CLI. No unwinding tables, no panic infrastructure, no exception objects. Gleam made this exact call for the same reasons. Rust's zero-cost Result analysis sourced from Stack Overflow.

5. **Module loading** — Lazy loading is the dominant win: Python PEP 690 measured **70% startup reduction and 40% memory reduction** on real CLIs. This is a concrete, primary-sourced number. Elixir's eager OTP loading is identified as the root problem.

6. **Macro/eval policy** — Compile-time only; ship no compiler in the runtime binary. Clojure's macro lifecycle (expansion time ≠ runtime) cited as the right mental model. Roc's approach cited as the production existence proof.

7. **Concurrency model** — Structured async/await for CLI (< 100 bytes per task), opt-in actor channels for TUI event loops. BEAM's 309 words/process cited from official Erlang docs. Goroutines' 2 KB stack compared for calibration.

**Decision table** covers all 10 dimensions with concrete startup/memory delta estimates.

**19 primary sources** kept, 5 dropped with explicit reasoning. Gaps section identifies what measurements would sharpen the recommendations.