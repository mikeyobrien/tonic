---
status: pending
HEARTBEAT_TASK_STATUS: todo
---

# Task: Fix LLVM CFG/PHI Inference Compile Failures

## Goal
Resolve control-flow lowering bugs that currently fail LLVM compilation for short-circuit/branch-heavy fixtures.

## Scope
- Fix MIR->LLVM/C CFG block-argument inference (`cannot infer block arg values`).
- Ensure short-circuit and branch forms produce valid native backend IR/codegen.
- Preserve deterministic behavior/diagnostics.

## Fixture Targets
- `examples/parity/02-operators/logical_keywords.tn`
- `examples/parity/02-operators/logical_short_circuit.tn`
- `examples/parity/06-control-flow/if_unless.tn`

## Deliverables
- Correct block arg/phi inference for affected CFG shapes.
- Regression tests for these control-flow forms in LLVM compile mode.

## Acceptance Criteria
- Listed fixtures compile with expected `check_exit` under LLVM backend.
- No regressions in existing LLVM compile suites.

## Verification
- Parity harness shows compile expectation match for listed fixtures.
- `cargo test --test compile_llvm_backend_control_flow_calls`
- `cargo test --test compile_aot_artifacts_cli`

## Suggested Commit
`fix(llvm): repair cfg block-arg inference for short-circuit and branch forms`
