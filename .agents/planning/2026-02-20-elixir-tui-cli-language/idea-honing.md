# Idea Honing

This document captures iterative requirements clarification Q&A.

## Q&A

### Q1
**Question:** Should we focus on language core design before TUI concerns, and which implementation toolchain should we use?

**Answer:** Yes — focus on the language first, then TUI. Use Rust.

### Q2
**Question:** For v0, how close should syntax be to Elixir?
- A) Near-identical Elixir syntax where practical
- B) Elixir-inspired but simplified syntax (fewer special cases)
- C) Minimal core syntax now, Elixir-compat layer later

**Answer:** B) Elixir-inspired but simplified syntax (fewer special cases).

**Rationale:** User explicitly chose B.

### Q3
**Question:** Should v0 target only Elixir-like syntax, or also Elixir runtime/BEAM compatibility?
- A) Syntax compatibility only; standalone Rust runtime and stdlib
- B) Syntax + partial BEAM interoperability
- C) High Elixir runtime compatibility (OTP/process model)

**Answer:** A) Syntax compatibility only; standalone Rust runtime and stdlib.

### Q4
**Question:** Which Elixir surface area must be supported in v0 for “LLMs can code day 1” to be true?
Pick must-have vs defer for each:
- `defmodule`, `def`, `if`, `case`, `cond`, `fn`
- pattern matching in function heads / `case`
- pipe operator `|>`
- `Enum`-style core collection functions
- structs/maps/keyword lists/tuples
- protocols
- macros (`defmacro`)
- `mix` compatibility

**Answer:** Initial preference was broad support, then refined in later decisions:
- Keep in v0: core syntax/forms, pattern matching, pipes, core collections, maps/tuples/keywords, protocols.
- Defer in v0: full `mix` compatibility and macros.

**Rationale:** Preserve LLM-friendly Elixir feel while controlling scope and runtime complexity.

### Q5
**Question:** What level of `mix` compatibility do you require in v0?
- A) Full compatibility with `mix.exs`, dependency resolution, tasks, and Hex workflows
- B) Source-compatible `mix.exs` for dependencies + basic tasks (`mix run/test`) only
- C) Syntax-compatible project file + our own package manager/runner commands (no full mix runtime behavior)

**Answer:** C) Syntax-compatible project file + our own package manager/runner commands (no full mix runtime behavior).

**Rationale:** Full mix compatibility is too much scope and runtime coupling for v0; language-core velocity and performance are higher priority.

### Q6
**Question:** Which features should be explicitly deferred from v0 to protect performance/scope?
- A) Macros (`defmacro`)
- B) Protocols
- C) Guards in pattern matching
- D) Full keyword-list semantics compatibility
- E) Dynamic code eval (`Code.eval_*` style)
- F) Process/actor runtime (OTP-like)
- G) None (keep all in v0)

**Answer:** Use default set: defer **A, E, F**.

**Rationale:** These have high implementation/runtime complexity and are not required for a fast, language-first v0.

### Q7
**Question:** Which v0 execution strategy should we choose for the language core?
- A) Tree-walking interpreter only (fastest to ship, slower runtime throughput)
- B) Tree-walking interpreter + on-disk compiled cache (best v0 balance)
- C) Bytecode VM from day one (higher effort, better throughput)

**Answer:** B) Tree-walking interpreter + on-disk compiled cache (best v0 balance).

**Rationale:** Best tradeoff for shipping quickly while preserving repeat-run performance.

### Q8
**Question:** What should the non-negotiable v0 performance gates be?
1. Cold start (simple script): A) <=20ms, B) <=50ms, C) <=100ms
2. Warm start (cache hit): A) <=5ms, B) <=10ms, C) <=20ms
3. Idle RSS after startup: A) <=20MB, B) <=30MB, C) <=50MB

**Answer:** Use suggested defaults: **1B, 2B, 3B**.

**Rationale:** Ambitious but realistic for v0 while still enforcing strong startup/memory discipline.

### Q9
**Question:** For v0 typing, which do you want?
- A) Dynamic typing (Elixir-style feel, fastest to ship)
- B) Gradual typing (optional type annotations + runtime checks)
- C) Static typing with inference (bigger compiler effort)

**Answer:** C) Static typing with inference (bigger compiler effort).

**Rationale:** User chose static typing with inference despite increased compiler complexity.

### Q10
**Question:** How strict should v0 static typing be?
- A) Strict by default: no implicit coercions, no `any`/dynamic escape hatch in v0
- B) Mostly strict: inference-first plus a small explicit `dynamic`/`any` escape hatch
- C) Permissive static: allow broad casts/coercions early, tighten later

**Answer:** B) Mostly strict: inference-first plus a small explicit `dynamic`/`any` escape hatch.

**Rationale:** Balances safety and ergonomics while keeping migration escape hatches for v0.

### Q11
**Question:** Which error model should be primary in typed v0?
- A) Result-first (`ok/err` + `?`), with panic only for unrecoverable faults
- B) Exception-first (`raise/rescue` style), with Result as optional library pattern
- C) Dual first-class model (both exceptions and Result equally central in v0)

**Answer:** A) Result-first (`ok/err` + `?`), with panic only for unrecoverable faults.

**Rationale:** Keeps control flow explicit, type-friendly, and lightweight for CLI-focused runtime goals.

### Q12
**Question:** Should requirements clarification stop here so I can draft the language-core design document and implementation plan?
- A) Yes, this is enough to move to design
- B) One more clarification pass first

**Answer:** A) Yes, this is enough to move to design.

**Rationale:** User approved moving from clarification to design.

### Q13
**Question:** Should acceptance verification use a dual-run BDD mechanism so scenarios can be executed both automatically and as agent-manual runs?

**Answer:** Yes. Use BDD as source-of-truth with dual-run modes.

**Rationale:** Keeps one acceptance definition while enabling both code execution and structured manual agent evidence.

## Connections
- [[rough-idea.md]]
