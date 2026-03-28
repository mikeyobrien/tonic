# Progress

## Current phase
Builder complete — ready for critic after slice commit.

## Active slice
Slice 1 — support `?`-suffixed predicate identifiers/calls and keep `Map.has_key?/2` working across run/compile parity.

## Builder checklist
- [x] Re-read `.miniloop/context.md`, `.miniloop/plan.md`, `.miniloop/progress.md`, and `.miniloop/ideas-report.md`
- [x] Add failing lexer/parser regressions first
- [x] Implement narrow lexer fix
- [x] Add parser regression coverage
- [x] Fix directly-blocking stdlib/native parity issues exposed by verification (`Map.has_key?/2` naming + `map_has_key` C host dispatch)
- [x] Verify targeted tests
- [x] Verify `check` on `.miniloop/logs/predicate-check.tn`
- [x] Verify manual `run` smoke
- [x] Verify manual `compile` + execute smoke
- [x] Record commit hash
- [ ] Emit `review.ready`

## Changed files
- `src/lexer/mod.rs`
- `src/lexer/tests.rs`
- `src/parser/tests.rs`
- `src/stdlib_sources.rs`
- `src/manifest_stdlib.rs`
- `src/c_backend/stubs_map.rs`
- `src/c_backend/stubs_host_path.rs`
- `tests/run_lazy_stdlib_loading_smoke.rs`
- `tests/runtime_llvm_map_predicate_smoke.rs`
- `.miniloop/context.md`
- `.miniloop/plan.md`
- `.miniloop/progress.md`

## Verification
Builder ran:
- `cargo fmt --check`
- `cargo test --bin tonic scan_tokens_supports_predicate_identifiers_before_call_parens`
- `cargo test --bin tonic scan_tokens_supports_predicate_atoms`
- `cargo test --bin tonic parse_ast_supports_predicate_function_defs_and_calls`
- `cargo test --bin tonic scan_tokens_supports_question_operator`
- `cargo test --bin tonic scan_tokens_char_literal_ascii_letter`
- `cargo test --bin tonic scan_tokens_char_literal_newline_escape`
- `cargo test --bin tonic parse_ast_supports_postfix_question_operator`
- `cargo test run_trace_supports_map_predicate_stdlib_function`
- `cargo test compiled_runtime_supports_map_has_key_predicate`
- `cargo run --bin tonic -- check .miniloop/logs/predicate-check.tn`
- `cargo run --bin tonic -- run .miniloop/logs/predicate-run.tn`
- `cargo run --bin tonic -- compile .miniloop/logs/predicate-run.tn --out .miniloop/logs/predicate-run-bin && ./.miniloop/logs/predicate-run-bin`

Observed outcomes:
- all listed commands passed
- interpreted smoke output: `true`
- compiled smoke output: `true`

## Relevant Issues
- `Ideas report P0 example claims missing C stubs for str_replace/sys_append_text, but current source already has them` — `out-of-scope`
  - Disposition: stale report context; not part of this slice.
- `Predicate-style function names like has_key?/2 fail because the lexer turns ?( into INT(40)` — `fix-now`
  - Disposition: fixed in `src/lexer/mod.rs` with lexer/parser regressions.
- `Potential ambiguity with postfix ? operator and char literals while adding predicate identifiers` — `fix-now`
  - Disposition: fixed narrowly and covered with targeted regression commands.
- `Predicate-style atoms and keyword-style keys like :ok? / exists?: were inconsistent` — `fix-now`
  - Disposition: fixed in lexer and parser regressions.
- `Map stdlib exported has_key/2 while interop and report expect Map.has_key?/2` — `fix-now`
  - Disposition: fixed in `src/stdlib_sources.rs` and `src/manifest_stdlib.rs`.
- `Compiled C backend lacked map_has_key host dispatch, breaking Map.has_key?/2 parity after compile` — `fix-now`
  - Disposition: fixed in `src/c_backend/stubs_map.rs` and `src/c_backend/stubs_host_path.rs`.
- `Broader stdlib predicate naming inconsistencies likely remain outside Map.has_key?/2 (for example other non-? predicate APIs)` — `fix-next`
  - Disposition: not required to make this slice green; revisit in a later ideas-report slice.
- `tests/runtime_llvm_map_stdlib_smoke.rs` currently hits a separate pre-existing compile failure (`tn_runtime_length` / `tn_runtime_elem` undeclared) on a much broader Map stdlib fixture` — `deferred`
  - Disposition: unrelated to the narrow predicate slice; builder used a dedicated compiled predicate smoke instead.

## Commit
- Slice commit: `HEAD` at `review.ready` time (`git rev-parse --short HEAD`)
- Review diff command: `git diff HEAD^ HEAD -- context.md plan.md progress.md src/lexer/mod.rs src/lexer/tests.rs src/parser/tests.rs src/stdlib_sources.rs src/manifest_stdlib.rs src/c_backend/stubs_map.rs src/c_backend/stubs_host_path.rs tests/run_lazy_stdlib_loading_smoke.rs tests/runtime_llvm_map_predicate_smoke.rs`
