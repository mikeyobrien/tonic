# Context

## Objective
Implement `code-task.formatter.md` incrementally. Slice 1 (`e81e939`) preserved comments in the live token formatter. Slice 2 (`aee0387`) added the standalone Wadler-Lindig algebra engine. Slice 3 (`d790d6c`) added the first library-only AST-to-doc path. Slice 4 (`44cd087`) extended that AST path over collection/data literals and `defstruct`. Slice 5 should add AST rendering for `case` branches and patterns so lowered control forms can format on the library-only path.

## Current repo state
- `tonic fmt` still routes through `src/formatter/mod.rs` -> `engine::format_source_inner`.
- `src/formatter/to_doc.rs` now supports module shells, `defstruct`, functions, calls/invokes, pipes, unary/binary ops, blocks, tuple/list/keyword/map/struct literals, map/struct updates, and direct `Expr::Case` rendering.
- The AST formatter now renders branch guards plus wildcard/bind/pin/literal/tuple/list (including cons-tail), map, and struct patterns, which is enough for current parser-lowered `if`/`unless`/`cond`/`with` coverage.
- The AST formatter still hard-errors on `Expr::Try`, `Expr::Raise`, `Expr::Fn`, `Expr::For`, interpolated strings, bitstrings, module attributes, and most non-`defstruct` module forms.
- Focused AST tests now prove deterministic rendering for direct `case` plus lowered control forms without touching CLI wiring.
- Live formatter behavior is intentionally unchanged; CLI/manual runs remain regression evidence only for this slice.

## Relevant code for slice 5
- `src/formatter/to_doc.rs` — add case-expression rendering plus pattern-to-doc helpers.
- `src/formatter/algebra.rs` — existing group/nest/line/soft-line primitives for stable case layout.
- `src/formatter/mod.rs` — must stay wired to `engine::format_source_inner` in this slice.
- `src/parser/ast/mod.rs` and `src/parser/ast/expr_def.rs` — `Expr::Case`, `CaseBranch`, and pattern enums.
- `tests/check_dump_ast_control_forms.rs` — shows how `if`/`unless`/`cond`/`with` lower to nested `case` AST.
- `tests/check_dump_ast_case_patterns.rs` — concrete tuple/list/map/literal pattern contracts.
- `tests/run_control_forms_smoke.rs` and `tests/run_case_pin_guard_match_smoke.rs` — semantic reference for the surface forms the AST formatter should eventually cover.
- `code-task.formatter.md` — source task; slice 5 should still stay library-only and avoid config/runtime wiring.

## Slice 5 scope target
Implement in this slice:
- `Expr::Case` rendering with `case <subject> do ... end` structure.
- Branch rendering with `pattern -> body` plus optional `when` guards.
- Pattern rendering for wildcard, bind, pin, literal, tuple, list, map, and struct patterns needed by current parser/control-form coverage.
- Stable formatting for nested case bodies so lowered `if`/`unless`/`cond`/`with` round-trip through the AST formatter.

Still intentionally deferred:
- wiring `format_source` / `tonic fmt` to parse ASTs.
- comment reinsertion in the AST formatter path.
- module attributes plus alias/import/require/use/protocol/defimpl forms.
- `try`/`rescue`/`catch`/`after`, `raise`, `for`, anonymous functions, string interpolation, and bitstrings.
- CLI/config work such as `--line-length` or `.tonic_formatter`.

## Verification evidence expected from critic
- `logs/formatter_to_doc_control.log` — focused AST formatter control-form/pattern tests.
- `logs/formatter_algebra.log` — algebra regression after any layout helper changes.
- `logs/formatter_regression.log` — live comment/idempotency regression.
- `logs/formatter_parity.log` — live CLI parity smoke regression.

## Critic constraint
If the builder keeps `tonic fmt` on `engine::format_source_inner`, there is still no honest manual smoke for the changed code path. CLI/manual runs remain regression evidence only, not proof that `src/formatter/to_doc.rs` executed.
