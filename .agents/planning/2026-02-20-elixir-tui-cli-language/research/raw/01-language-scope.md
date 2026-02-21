Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/01-language-scope.md` — 307 lines.

---

Here's what landed and what I flagged as uncertain:

### Key findings

**The problem is real and well-documented.** `elixir script.exs` carries 300–500 ms of BEAM cold-start overhead even for trivial scripts. The Elixir community openly calls it a CLI non-starter. Babashka proved in the Clojure world that the niche — same language syntax, no VM tax — is large enough to have a thriving ecosystem.

**Babashka's architecture is the template.** It ships a GraalVM native image that wraps SCI (a Clojure subset interpreter). That combination gets ~14–30 ms startup with ~60 MB memory. An Elixir-syntax equivalent needs the same two decisions made: (1) interpreter or compiler, (2) native image strategy (GraalVM, Rust binary, etc.). This research doesn't resolve that — it's track 2.

**Performance KPIs set conservatively:**
- Cold start: ≤20 ms (target), ≤50 ms (commitment)
- Memory idle: ≤30 MB
- TUI frame rate: ≥60 fps / ≤16 ms render

**v0 non-goals are load-bearing.** The explicit cuts — no OTP, no macros, no BEAM compatibility, no distribution — are what make a v0 feasible. Macros especially: Elixir's `defmacro` is AST-manipulation at compile time and requires a full compiler; deferring it is correct.

**Explicit uncertainties called out:**
- Babashka's throughput vs Python has no hard public benchmark (stated design intent only)
- The 14 ms startup figure is from 2020 GraalVM — needs fresh measurement
- Implementation runtime strategy is unresolved (biggest open question for the whole project)
- "Elixir syntax" conflates syntax and semantics — needs a formal compatibility matrix