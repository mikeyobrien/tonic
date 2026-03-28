# Plan

## Critic verification checklist
1. Inspect the landed slice only.
   - Use the slice commit hash from `.miniloop/progress.md`.
   - Preferred diff command: `git diff HEAD^ HEAD -- context.md plan.md progress.md src/lexer/mod.rs src/lexer/tests.rs src/parser/tests.rs src/stdlib_sources.rs src/manifest_stdlib.rs src/c_backend/stubs_map.rs src/c_backend/stubs_host_path.rs tests/run_lazy_stdlib_loading_smoke.rs tests/runtime_llvm_map_predicate_smoke.rs`

2. Re-run builder verification exactly.
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

3. Manual smoke, independently.
   - `cargo run --bin tonic -- run .miniloop/logs/predicate-run.tn`
   - `cargo run --bin tonic -- compile .miniloop/logs/predicate-run.tn --out .miniloop/logs/predicate-run-bin && ./.miniloop/logs/predicate-run-bin`
   - Expected output for both execution paths: `true`

4. Review for narrowness.
   - Plain identifier trailing `?` should only bind at unambiguous boundaries.
   - Atom handling should not affect postfix operator semantics.
   - Native fix should be limited to `map_has_key` support.

## Expected verdict criteria
- Pass if lexer/parser regressions, interpreted smoke, and compiled smoke all succeed and the diff stays narrow.
- Reject if postfix `?` or char literals regress, or if `Map.has_key?/2` still diverges between run and compile.
