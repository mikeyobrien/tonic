# Context

## Objective
Implement `.miniloop/ideas-report.md` with one concrete slice at a time.

## Completed slice
Slice 1 — support `?`-suffixed predicate identifiers and atoms, and keep the newly-usable `Map.has_key?/2` path working in both interpreted and compiled execution.

## What changed
- `src/lexer/mod.rs`
  - added narrow trailing-`?` identifier scanning rules
  - plain identifiers absorb `?` only for unambiguous boundaries: `(`, `/`, `:`
  - atoms absorb trailing `?` before atom-safe boundaries
- `src/lexer/tests.rs`
  - added predicate identifier and atom lexer regressions
- `src/parser/tests.rs`
  - added parser regression for predicate defs/calls, predicate atoms, and keyword-style `exists?` map keys
- `src/stdlib_sources.rs`
- `src/manifest_stdlib.rs`
  - renamed `Map.has_key/2` stdlib entry to `Map.has_key?/2`
- `src/c_backend/stubs_map.rs`
- `src/c_backend/stubs_host_path.rs`
  - added native compiled dispatch for `map_has_key`
- `tests/run_lazy_stdlib_loading_smoke.rs`
  - added interpreted stdlib smoke for `Map.has_key?/2`
- `tests/runtime_llvm_map_predicate_smoke.rs`
  - added compiled stdlib smoke for `Map.has_key?/2`

## Why the slice widened slightly
The lexer/parser fix made `Map.has_key?(...)` parse, but verification exposed two directly relevant parity gaps:
1. stdlib still exported `Map.has_key/2` instead of `Map.has_key?/2`
2. compiled C backend lacked `map_has_key` host dispatch

Both blocked the required `check`/`run`/`compile` verification path for the predicate example, so they were fixed in the same slice.

## Intended boundary
This slice is intentionally narrow.
- Supported now:
  - `def has_key?(...)`
  - `Map.has_key?(...)`
  - `:ok?`
  - `%{exists?: true}`
  - `&Map.has_key?/2`
- Intentionally unchanged:
  - postfix question operator parsing (`value()?`, `1?`, `x? y`)
  - char literals (`?a`, `?\n`, `?0`)
  - ambiguous bare/no-paren `name? value` forms

## Critic focus
- Verify the lexer did not steal postfix `?` or char literals.
- Verify `Map.has_key?/2` now works in both `tonic run` and `tonic compile` paths.
- Review only the slice diff/commit, not the unrelated dirty worktree.
