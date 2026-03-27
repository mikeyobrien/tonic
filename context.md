# Context

## Objective
Implement `code-task.formatter.md` incrementally. Slice 1 (`e81e939`) preserved comments in the live token formatter. Slice 2 (`aee0387`) added the standalone Wadler-Lindig algebra engine. Slice 3 now adds the first AST-driven formatter path without switching the CLI over yet.

## Current repo state
- `tonic fmt` still routes through `src/formatter/mod.rs` -> `engine::format_source_inner`.
- `src/formatter/algebra.rs` now includes the original group/nest/flex primitives plus `SoftLine`, which the new converter uses to render flat calls as `foo(arg)` and broken calls as one-arg-per-line.
- `src/formatter/to_doc.rs` now exists as a library-only AST-to-doc converter with a `format_parsed_source` / `format_ast` test entrypoint.
- The parser AST under `src/parser/ast/` supplies the narrow slice-3 surface already covered here: modules, functions, identifier params/defaults, guards, calls, pipes, blocks, and simple literals.
- Live formatter behavior is intentionally unchanged; comment preservation and CLI formatting still come from the token formatter.

## Relevant code for slice 3
- `src/formatter/mod.rs` — wires in `to_doc` but keeps `format_source` on `engine`.
- `src/formatter/algebra.rs` — reusable `Doc` tree and renderer; `SoftLine` was added here to support zero-or-newline call boundaries.
- `src/formatter/to_doc.rs` — new AST-to-doc converter and focused unit tests.
- `src/parser/mod.rs` — public `parse_ast` entrypoint; there is still no `src/parser.rs`.
- `src/parser/ast/mod.rs` — module/function/pattern/shared AST types.
- `src/parser/ast/expr_def.rs` + `src/parser/ast/expr_impl.rs` — expression variants/accessors consumed by the converter.
- `code-task.formatter.md` — source task; slice 3 still covers only the first narrow AST-to-doc surface.

## Slice 3 scope shipped
Implemented in this slice:
- top-level module rendering
- `def` / `defp` function headers
- identifier parameters, including defaults
- optional function guards
- simple expression bodies used by focused tests: variables, literals, calls, pipe chains, and blocks
- width-sensitive call and pipe rendering through the algebra layer

Still intentionally deferred:
- wiring `format_source` / `tonic fmt` to parse ASTs
- comment reinsertion in the AST formatter path
- maps, structs, case/cond/if/unless, try/rescue, comprehensions, anonymous functions
- non-trivial function-head patterns
- CLI/config work such as `--line-length` or `.tonic_formatter`

## Verification evidence expected from critic
- `logs/formatter_to_doc.log` — focused converter tests
- `logs/formatter_algebra.log` — algebra regression after adding `SoftLine`
- `logs/formatter_regression.log` — live comment/idempotency regression
- `logs/formatter_parity.log` — live CLI parity smoke regression

## Critic constraint
If the builder keeps `tonic fmt` on `engine::format_source_inner`, there is still no honest manual smoke for the changed code path. CLI/manual runs remain regression evidence only, not proof that `src/formatter/to_doc.rs` executed.
