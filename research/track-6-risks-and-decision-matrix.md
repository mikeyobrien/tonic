# Research: Track 6 — Elixir-Syntax Fast CLI/TUI Language: Risks, Constraints & Decision Matrix

## Summary

Building a language with Elixir's syntax targeting fast CLI/TUI execution is technically
feasible but carries high implementation risk in three clusters: (1) Elixir's grammar is
genuinely ambiguous and requires GLR-level machinery to parse correctly, (2) CLI tooling
demands sub-100ms startup times that BEAM and most VMs cannot meet without native
compilation or AOT, and (3) the full-stack tooling burden (LSP, formatter, debugger,
package index) is a multi-person-year commitment that commonly kills language projects
before they reach adoption.

---

## Findings

### 1. Parsing Complexity — Elixir Is Harder to Parse Than It Looks

**1a. Grammar ambiguity is structural, not incidental.**  
Elixir's syntax has multiple genuine ambiguities: `do`/`end` blocks vs `do:` keyword
pairs, unary vs binary operator forms (e.g., `+` as prefix or infix), parentheses-as-call
vs parentheses-as-group, zero-arity calls requiring parens to avoid variable shadowing, and
`sigils`. The official Elixir tree-sitter grammar documentation explicitly acknowledges
using an external C scanner *and* GLR fallback (conflict resolution via dynamic precedence)
to handle cases that no LALR(1) or LL(k) grammar can cleanly express.

> "Whenever the parser stumbles upon this conflict it uses its GLR algorithm, basically
> considering both interpretations until one leads to parsing error."  
> — [tree-sitter-elixir/docs/parser.md](https://github.com/elixir-lang/tree-sitter-elixir/blob/main/docs/parser.md)

**1b. Operator overloading compounds the problem.**  
Elixir allows operators to be redefined per module. A parser that aims for correct
static semantics must track `import`/`use` scope to know which `+` is in effect.
The Elixir syntax reference documents that `add (1, 2)` (space before paren) is a
syntax *error* while `add(1, 2)` is a valid call — a whitespace-sensitive disambiguation
rule that hand-rolled and generated parsers alike must encode explicitly.  
[Elixir Syntax Reference](https://hexdocs.pm/elixir/syntax-reference.html)

**Implication:** A correct, complete parser for Elixir syntax needs either tree-sitter
(with external scanner + GLR), a hand-written recursive-descent parser with backtracking,
or a PEG parser. A yacc/LALR approach alone will not close the gap.

---

### 2. Performance — BEAM Startup Is Disqualifying for CLI Use Cases

**2a. BEAM baseline latency: ~260–400ms.**  
A 2014 Erlang mailing list measurement (still directionally accurate today) showed BEAM
startup at ~260ms for R13, degrading to ~375ms by R16. Modern OTP is similar: loading
~2,000 modules before reaching user code is the structural cost.  
[erlang-questions: beam.smp startup time regression](http://erlang.org/pipermail/erlang-questions/2014-April/078476.html)

**2b. The CLI bar is ≤ 50ms.** Tools like `ripgrep`, `fd`, `bat`, `jq` — the reference
class for modern CLI — all start in single-digit milliseconds on modern hardware. A
200–400ms cold-start is perceived as "sluggish" for interactive TUI use and disqualifying
for shell completions, hooks, or scripted loops.

**2c. Native/AOT compilation achieves sub-10ms.**  
GraalVM Native Image drops JVM-class startup from 3–4s to under 100ms; Rust/Go binaries
start in <5ms. Crystal (LLVM-backed, Ruby-influenced syntax) demonstrates that a high-level
language targeting native compilation can achieve C-tier startup.  
[GraalVM Native Image: Java's Answer to Rust's Startup Speed](https://www.javacodegeeks.com/2026/02/graalvm-native-image-javas-answer-to-rusts-startup-speed.html)

**Implication:** Any architecture that relies on BEAM, JVM, or an interpreted VM as the
*execution* layer for CLI user code fails the startup constraint unless a warm-daemon model
is used (significant complexity, fragile for scripting contexts).

---

### 3. Macro System Semantics — Surface Syntax ≠ Full Elixir

**3a. Elixir macros operate on the quoted AST.**  
Elixir's `defmacro` / `quote` / `unquote` system is deeply tied to the BEAM's compile-time
expansion pipeline. Hygiene is implemented by tracking variable context through module
scope, and it can be bypassed with `var!/2` and `alias!/1`. Replicating this in a new
compiler means either (a) dropping macros, (b) building a full compile-time evaluation
layer, or (c) implementing a subset that is explicitly not Elixir-compatible.  
[Elixir Macros — hexdocs](https://hexdocs.pm/elixir/macros.html)

**3b. Metaprogramming is load-bearing in the ecosystem.**  
`Phoenix`, `Ecto`, `LiveView`, `NimbleOptions` — major Elixir libraries make heavy use of
macros for DSL construction. A language that looks like Elixir but doesn't support macros
cannot consume Hex packages, which removes the primary ecosystem benefit of syntax
compatibility.

**Implication:** The project must pick a lane — either implement macro expansion (2–6
months of dedicated work) or explicitly brand as "Elixir-*inspired* syntax, no macros"
and accept ecosystem isolation.

---

### 4. Tooling Burden — LSP, Formatter, Debugger Are Underestimated

**4a. LSP is a full project.**  
The Language Server Protocol decouples language intelligence from editors, but implementing
a correct LSP server (completions, diagnostics, go-to-definition, hover, rename) for a new
language typically takes 6–18 months of full-time engineering. Without it, developer
experience is a competitive disadvantage vs. established languages.  
[Language Server Protocol — Wikipedia](https://en.wikipedia.org/wiki/Language_Server_Protocol)

**4b. Formatter, package manager, and debugger add further time.**  
A realistic estimate for a "day-1 usable" tooling stack:

| Tool         | Estimated effort    |
|--------------|---------------------|
| Lexer+parser | 2–4 months          |
| Type checker | 3–9 months          |
| Code generator (native) | 3–12 months |
| LSP server   | 6–18 months         |
| Formatter    | 1–2 months          |
| Debugger     | 3–6 months          |
| Package registry | 2–4 months      |
| **Total**    | **~20–55 months**   |

A solo or two-person team cannot deliver this in under 3 years without strategic shortcuts
(transpilation to avoid codegen, reuse of existing formatters/linters, targeting one OS).

---

### 5. Cross-Platform Terminal Behavior — Windows Is the Fault Line

**5a. ANSI/VT100 support on Windows is partial and version-gated.**  
Windows 10 1511+ supports VT sequences via `ENABLE_VIRTUAL_TERMINAL_PROCESSING`, but
many real-world Windows environments (CI runners, corporate terminals, older builds, `cmd`
vs PowerShell vs Windows Terminal) have inconsistent behavior. Termion and rustbox are
POSIX-only. Crossterm (Rust) and Terminal.Gui (.NET) are the two viable cross-platform
TUI abstraction layers, both with known issues in non-standard terminals.  
[Stack Overflow — Win32 ANSI/VT100](https://stackoverflow.com/questions/16755142/how-to-make-win32-console-recognize-ansi-vt100-escape-sequences-in-c)

**5b. Windows raw mode requires Win32 API, not termios.**  
Any TUI runtime that assumes `termios` (POSIX) for raw terminal access must have a
separate Win32 `CONSOLE_MODE` code path. This doubles the QA surface and requires
Windows CI from day one to avoid regressions.

**5c. Ratatui (Rust) is the current best-practice answer.**  
Ratatui builds on crossterm and provides a retained-mode widget model. If the language's
TUI primitives are implemented *in Rust using ratatui*, they inherit the cross-platform
work for free — but this constrains the implementation language to Rust (or requires a
C FFI bridge).

---

### 6. Ecosystem and Adoption Risk

**6a. Gleam shows the path — and its ceiling.**  
Gleam v1.0 (March 2024) is the closest precedent: Erlang-ecosystem language, static types,
modern syntax. It runs on BEAM and compiles to JS. Four years to v1.0, strong engineering
lead, Thoughtworks "Assess" ring by April 2025. It solves *type safety on BEAM*, not *fast
CLI startup*. Its interop story (can use Elixir deps) is a meaningful differentiator that
a new language lacking BEAM interop cannot claim.  
[Gleam 1.0 — InfoQ](https://www.infoq.com/news/2024/03/gleam-erlang-virtual-machine-1-0/)

**6b. Differentiation must be sharp.**  
An InfoQ analysis of language adoption identifies the core requirement: the language must
solve a problem that existing languages provably cannot, at acceptable switching cost. "Like
Elixir but faster startup" is a coherent value prop *only if* the language preserves enough
Elixir-ness that current Elixir users see a migration path.  
[When and How to Win with New Programming Languages — InfoQ](https://www.infoq.com/presentations/adopt-new-programming-language/)

---

## Architecture Paths — Decision Matrix

Three viable paths, scored on six weighted criteria (higher = better):

| Criterion                        | Weight | Path A: Native (LLVM) | Path B: Transpile → Go | Path C: Bytecode VM + JIT |
|----------------------------------|--------|-----------------------|-----------------------|--------------------------|
| **Startup performance** (<50ms)  | 25%    | 9 — native binary <5ms | 8 — Go binary <10ms | 2 — VM boot 100–500ms |
| **Implementation velocity**      | 20%    | 3 — LLVM IR, codegen, linking = 12–24mo | 7 — emit Go, `go build` | 6 — simpler than LLVM |
| **Semantic fidelity to Elixir**  | 20%    | 8 — can implement any semantics | 5 — pattern match, pipes map OK; process model doesn't | 8 — can model processes |
| **Ecosystem/tooling leverage**   | 15%    | 4 — build everything | 7 — inherit Go stdlib, go vet, gofmt | 3 — build everything |
| **Cross-platform TUI support**   | 10%    | 6 — use crossterm via Rust, or C FFI | 8 — Go's `tcell` / `bubbletea` is mature | 5 — depends on runtime |
| **Kill risk** (lower = safer)    | 10%    | 7 — hard to abandon mid-way | 8 — incremental, can stop at "good enough" | 5 — VM rewrites are common failure mode |

**Weighted scores:**

| Path                 | Score |
|----------------------|-------|
| Path B: Transpile → Go | **6.65** |
| Path A: Native LLVM   | **5.95** |
| Path C: Bytecode VM   | **4.80** |

> Path B (transpile to Go) wins primarily on velocity and startup, while Path A (native
> LLVM) is the correct long-term answer if the team grows or performance requirements
> tighten beyond what Go can deliver. Path C is only defensible if process-model semantics
> (lightweight millions-of-processes concurrency) are a hard requirement.

**Variant: Transpile → Rust (Path B')**  
Rust as target instead of Go trades 2x implementation velocity for 2x better runtime
performance and memory safety. Worth considering if TUI raw-terminal work is heavy (no
crossterm integration cost). Scored ~6.3 — between B and A. The LogRocket analysis of the
TypeScript compiler rewrite covers this tradeoff precisely:  
[Why Go Wasn't the Right Choice for the TypeScript Compiler](https://blog.logrocket.com/go-wrong-choice-typescript-compiler/)

---

## Kill Criteria

Stop the project if any of the following are true at the stated checkpoint:

| # | Criterion | Checkpoint |
|---|-----------|------------|
| K1 | Parser cannot correctly round-trip 95% of the Elixir standard library source (excluding macros) | Month 3 |
| K2 | "Hello world" binary startup >100ms on any supported platform | Month 4 |
| K3 | Pipe operator (`\|>`) and pattern matching semantics cannot be correctly expressed in the target (transpile) or IR (native) | Month 5 |
| K4 | Full tooling stack (parser + codegen + LSP MVP) estimate exceeds 3 person-years with current team size | Month 6 |
| K5 | No 10+ user community or external contributors after 12 months of public availability | Month 18 |
| K6 | An existing language (Gleam, Crystal, or a new entrant) ships Elixir-syntax CLI support covering ≥70% of the use case | Ongoing |

---

## Mitigation Options

| Risk | Primary Mitigation | Fallback |
|------|--------------------|---------|
| Parser complexity | Use tree-sitter-elixir grammar as base (Apache 2.0); add custom rules only for divergence | Hand-write recursive-descent with backtracking for ambiguous forms |
| BEAM startup (if BEAM path chosen) | Port-as-daemon with Unix socket IPC; clients are thin Go/C callers | Abandon BEAM; AOT via GraalVM Native Image (~100ms viable) |
| Macro system | Drop macros in v1; implement `defmacro` as a tracked future milestone; document explicitly | Implement a safe subset (no `var!/2`); ship an AST transformation API |
| LSP burden | Use `tower-lsp` (Rust) or `lsp4go` crate/module as scaffolding; implement diagnostics-only first | Ship VS Code extension with syntax highlighting only; defer semantics |
| Windows terminal | Target crossterm (Rust) or bubbletea/tcell (Go) from day one; no termios | Ship Linux/macOS only for v1 with documented Windows roadmap |
| Ecosystem isolation | Define a narrow "Elixir-syntax, no BEAM" contract; target scripting (pipelines, CLI tools) not app development | Add an FFI to call Elixir Mix projects at the process boundary |

---

## Sources

**Kept:**
- [tree-sitter-elixir/docs/parser.md](https://github.com/elixir-lang/tree-sitter-elixir/blob/main/docs/parser.md) — Primary technical reference on Elixir grammar ambiguities, GLR use, external scanner
- [Elixir Syntax Reference — hexdocs](https://hexdocs.pm/elixir/syntax-reference.html) — Authoritative spec on call vs variable ambiguity, operator precedence
- [Elixir Macros — hexdocs](https://hexdocs.pm/elixir/macros.html) — Defines hygiene model and var!/2 escape hatch
- [erlang-questions: BEAM startup regression](http://erlang.org/pipermail/erlang-questions/2014-April/078476.html) — Measured BEAM cold-start baselines (260–375ms)
- [GraalVM Native Image startup — JavaCodeGeeks 2026](https://www.javacodegeeks.com/2026/02/graalvm-native-image-javas-answer-to-rusts-startup-speed.html) — AOT startup benchmark reference
- [Gleam 1.0 — InfoQ](https://www.infoq.com/news/2024/03/gleam-erlang-virtual-machine-1-0/) — Nearest precedent project; timeline and scope reference
- [Why Go Wasn't Right for TypeScript Compiler — LogRocket](https://blog.logrocket.com/go-wrong-choice-typescript-compiler/) — Transpile target tradeoff analysis (Go vs Rust)
- [Resilient LL Parsing Tutorial — Lobsters](https://lobste.rs/s/lbzdtu/resilient_ll_parsing_tutorial) — Modern parser strategy overview (GLR, tree-sitter context)
- [When and How to Win with New PL — InfoQ](https://www.infoq.com/presentations/adopt-new-programming-language/) — Adoption dynamics and kill criteria framing
- [Language Server Protocol — Wikipedia](https://en.wikipedia.org/wiki/Language_Server_Protocol) — LSP scope and effort framing
- [Stack Overflow: Win32 ANSI/VT100](https://stackoverflow.com/questions/16755142/how-to-make-win32-console-recognize-ansi-vt100-escape-sequences-in-c) — Windows terminal compatibility evidence

**Dropped:**
- zigpoll.com "emerging languages 2024" — SEO roundup, no primary evidence, low signal
- rankred.com "15 New Languages 2025" — marketing content, no technical depth
- Reddit threads (r/golang, r/rust) — useful color but not citable as primary evidence; subsumed by LogRocket analysis
- Wellnuts "Elixir Macros" blog — redundant with hexdocs official docs

---

## Gaps

1. **Actual parser throughput benchmarks** for tree-sitter-elixir on large codebases
   (e.g., Phoenix source) are not publicly documented. Next step: clone and benchmark
   locally with `npx tree-sitter parse --stat`.

2. **TUI runtime on Windows with Go's bubbletea/tcell** — real-world behavior on Windows
   Terminal vs. cmd vs. ConEmu is documented via issues but no systematic compatibility
   matrix exists. Suggests building a simple canary test suite.

3. **Transpile fidelity for Elixir pattern matching** — how well does Elixir destructuring
   (multi-head functions, guard clauses, pin operator `^`) map onto Go's type-switch or
   struct matching. No prior art found; requires a prototype spike.

4. **Community precedents for "Elixir syntax, not BEAM" languages** — no language that
   retains Elixir syntax but compiles natively was found in the search. This is either a
   gap in the market or a signal that the Elixir community considers the BEAM non-optional.

5. **Track 6 scope definition** — this research assumes "fast CLI/TUI language with Elixir
   syntax" as the full brief. If Track 6 has additional constraints (e.g., embeddable
   scripting, WASM target, specific domain), the matrix weights would shift materially.
