---
status: completed
started: 2026-02-24
completed: 2026-02-24
HEARTBEAT_TASK_STATUS: done
---

# Task: Close LLVM Compile Blockers for Literals and Call Surface

## Goal
Eliminate literal/call-surface compile failures preventing LLVM backend from compiling active catalog fixtures.

## Scope
- Add lowering support required for:
  - `const_string`
  - `const_float`
- Ensure generated C/native path handles map access call signatures correctly where currently mismatched.
- Keep diagnostics deterministic for unsupported operations that remain intentionally unsupported.

## Fixture Targets (must compile with expected `check_exit`)
- `examples/bools_and_nils.tn`
- `examples/parity/01-literals/bool_nil_string.tn`
- `examples/parity/01-literals/float_and_int.tn`
- `examples/parity/01-literals/heredoc_multiline.tn`
- `examples/parity/01-literals/interpolation_basic.tn`
- `examples/parity/02-operators/concat_and_list_ops.tn`
- `examples/parity/03-collections/map_dot_and_index_access.tn`
- `examples/parity/08-errors/host_call_and_protocol_dispatch.tn`
- `examples/parity/08-errors/ok_err_constructors.tn`
- `examples/parity/08-errors/question_operator_err_bubble.tn`
- `examples/parity/99-stretch/sigils.tn`

## Deliverables
- Updated lowering/codegen for string/float literals.
- Fixed generated C/runtime call signatures for map access path.
- Regression tests for these fixtures under LLVM compile mode.

## Acceptance Criteria
- All listed fixtures match catalog `check_exit` under `compile --backend llvm`.
- No C compiler arity/signature errors in generated output for map access fixtures.
- Existing compile diagnostics remain deterministic.

## Verification
- Run new parity harness and confirm these fixtures leave compile-mismatch bucket.
- Run targeted compile tests + `cargo test --test compile_aot_artifacts_cli`.

## Suggested Commit
`feat(llvm): add literal lowering and map-access compile parity fixes`
