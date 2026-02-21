# Task: Tooling Parity Sweep (fmt/diagnostics/docs fixtures)

## Goal
Close parity loop with stable developer workflow: formatting, diagnostics, and translated fixtures.

## Scope
- Replace `fmt` skeleton with functional formatter baseline.
- Improve diagnostic quality (span + actionable hints) for unsupported/misused syntax.
- Add Elixir-doc-style translation fixtures that must parse/run/check.
- Update parity checklist statuses and evidence.

## Out of Scope
- Full ExDoc clone.
- OTP/runtime-distribution features.

## Deliverables
- Functional `tonic fmt <path>` behavior.
- Regression suite validating translated syntax examples.
- Updated docs: checklist + parity status report.

## Acceptance Criteria
- Formatter command performs deterministic rewrites/check mode behavior.
- Error messages for new syntax are deterministic and actionable.
- Fixture suite demonstrates practical snippet portability.

## Verification
- `cargo test`
- `cargo run -- fmt <fixture>` + formatter idempotence tests.

## Suggested Commit
`feat(parity): finalize tooling parity and fixture-based compatibility sweep`
