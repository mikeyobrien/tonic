# Task: Standardize Self-Hosted Lexer Machine-Readable Output

## Description
Add a first-class machine-readable output contract for the self-hosted lexer so the parity harness no longer depends on the `TONIC_SYSTEM_LOG_PATH` structured-log side channel. The goal is to make the self-hosted lexer expose structured token output directly through a stable CLI/stdout path while preserving the current partial self-hosting milestone framing and exact token parity requirements.

## Background
The current self-hosted lexer milestone is real and test-backed, but its machine-readable contract is still slightly indirect. The Rust reference lexer emits structured JSON through `tonic check <path> --dump-tokens --format json`, while the self-hosted lexer currently emits a structured `self_hosted_lexer.tokens` event via `System.log(...)` and the parity harness reads that log file through `TONIC_SYSTEM_LOG_PATH`.

That is better than scraping Tonic map/list rendering, but it is still not the cleanest contract. The remaining hardening work is to expose a direct structured output path for the self-hosted lexer and update the harness/tests/docs accordingly.

## Reference Documentation
**Required:**
- Design: `.agents/planning/2026-03-07-tonic-primitive-kernel/design/detailed-design.md`

**Additional References (if relevant to this task):**
- `.agents/planning/2026-03-07-tonic-primitive-kernel/implementation/plan.md`
- `docs/self-hosting-status.md`
- `tests/common/self_hosted_lexer_parity.rs`
- `tests/run_self_hosted_lexer_scaffold_example.rs`

**Note:** You MUST read the detailed design document before beginning implementation. Read additional references as needed for context.

## Technical Requirements
1. Define a stable machine-readable output contract for the self-hosted lexer that can be consumed directly from CLI/stdout instead of through `TONIC_SYSTEM_LOG_PATH`.
2. Preserve exact parity semantics on `kind`, `lexeme`, `span_start`, and `span_end`.
3. Update the self-hosted lexer entrypoint and/or CLI surface so structured output is explicit and deterministic.
4. Update the parity harness to consume the direct structured contract rather than the structured-log side channel.
5. Preserve or intentionally document any remaining human-readable output behavior so existing user-facing usage is not accidentally muddled.
6. Update tests and docs to reflect the direct machine-readable contract and keep milestone wording honest.

## Dependencies
- Existing structured reference token dump in `src/main.rs` / `--dump-tokens --format json`
- Current self-hosted lexer project in `examples/apps/self_hosted_lexer`
- Current parity harness in `tests/common/self_hosted_lexer_parity.rs`
- Current milestone/status docs in `docs/self-hosting-status.md`

## Implementation Approach
1. Inspect the current self-hosted lexer output path and choose the narrowest direct structured output shape that aligns with the reference lexer contract.
2. Implement the direct structured output path in the self-hosted lexer project without widening scope into parser or broader self-hosting work.
3. Replace the harness dependency on `TONIC_SYSTEM_LOG_PATH` with direct structured parsing from the new output contract.
4. Update scaffold and parity tests to validate the new contract and preserve exact token equality.
5. Refresh status/docs wording so the repo describes the machine-readable contract accurately and no more strongly than the evidence supports.

## Acceptance Criteria

1. **Direct Structured Output Contract**
   - Given the self-hosted lexer is run in machine-readable mode
   - When it processes a source fixture
   - Then it emits a stable structured token payload directly through CLI/stdout without requiring `TONIC_SYSTEM_LOG_PATH`

2. **Exact Token Parity Preserved**
   - Given a curated lexer parity fixture
   - When the Rust reference lexer dump and self-hosted lexer dump are compared
   - Then the harness verifies exact equality on `kind`, `lexeme`, `span_start`, and `span_end`

3. **Harness No Longer Depends on Structured Log Side Channel**
   - Given the parity harness implementation
   - When it gathers self-hosted lexer output
   - Then it reads the direct machine-readable contract instead of scraping structured log events from `TONIC_SYSTEM_LOG_PATH`

4. **Scaffold and Regression Coverage Updated**
   - Given the self-hosted lexer scaffold and parity tests
   - When the targeted test suite is run
   - Then tests cover the new direct structured contract and continue to pass for the curated parity corpus

5. **Docs Stay Honest**
   - Given the self-hosting status documentation
   - When the new output contract is documented
   - Then the docs describe the direct machine-readable path clearly without overstating the milestone as full self-hosting

6. **Unit/Targeted Test Coverage**
   - Given the touched self-hosted lexer and parity harness surfaces
   - When the targeted verification commands are run
   - Then the relevant tests for structured output, scaffold behavior, and curated parity pass successfully

## Metadata
- **Complexity**: Medium
- **Labels**: Self-Hosting, Lexer, Parity, CLI, JSON, Test Harness
- **Required Skills**: Rust CLI integration, Tonic project structure, test harness design, JSON contracts, compiler frontend parity
