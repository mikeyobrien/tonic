# Tonic Self-Hosted Lexer Hardening Loop

## Objective

Take the current self-hosted lexer milestone from "real but thin" to "credible and maintainable."

The current implementation already landed:

- `String.to_charlist/1`
- structured Rust token dump via `tonic check <path> --dump-tokens --format json`
- a self-hosted lexer project at `examples/apps/self_hosted_lexer`
- a parity harness and status docs

This loop should fix the three obvious weaknesses:

1. the curated parity corpus is too small to justify a strong milestone claim,
2. the self-hosted lexer output contract is hacky because the harness parses Tonic text output instead of comparing structured JSON on both sides,
3. `examples/apps/self_hosted_lexer/src/compiler/lexer.tn` is too large and needs to be split into smaller implementation files.

## Source-of-truth context

Read these first:

- `AGENTS.md`
- `docs/self-hosting-status.md`
- `.agents/planning/2026-03-07-tonic-primitive-kernel/design/detailed-design.md`
- `.agents/planning/2026-03-07-tonic-primitive-kernel/implementation/plan.md`
- `tests/self_hosted_lexer_parity.rs`
- `tests/common/self_hosted_lexer_parity.rs`
- `tests/run_self_hosted_lexer_scaffold_example.rs`
- `examples/apps/self_hosted_lexer/src/main.tn`
- `examples/apps/self_hosted_lexer/src/compiler/lexer.tn`
- `src/main.rs`
- `src/lexer.rs`

Use repo source as truth if docs and implementation diverge.

## Problem statement

The current milestone is good enough to keep, but not clean enough to call finished.

### Current issues

#### 1. Curated parity corpus is too thin

The current curated parity gate appears to rely on only one fixture under:

- `tests/fixtures/self_hosted_lexer_parity/`

That is not enough evidence for a serious parity-verified milestone.

#### 2. Self-hosted lexer output contract is brittle

The Rust reference lexer emits structured JSON.
The self-hosted lexer still emits Tonic map/list text that the harness parses manually.

That should be replaced with a direct structured JSON contract on the self-hosted side so the harness compares the same shape on both sides.

#### 3. Self-hosted lexer implementation file is too large

This file is over 1000 lines:

- `examples/apps/self_hosted_lexer/src/compiler/lexer.tn`

That violates the repo preference for smaller implementation files and makes future work worse.

## Scope

### In scope

- expanding the curated self-hosted lexer parity corpus into a genuinely useful milestone set
- converting the self-hosted lexer output to the same structured JSON-style contract expected by the parity harness
- simplifying the parity harness once JSON-to-JSON comparison exists
- splitting the self-hosted lexer into smaller, coherent modules/files
- updating docs/status wording if the strengthened evidence changes what can honestly be claimed
- adding regression tests and parity fixtures for the broadened curated corpus

### Out of scope unless strictly required

- broad new self-hosting work beyond lexer hardening
- parser/typechecker/backend self-hosting
- broad stdlib expansion
- release/distribution work for prebuilt binaries
- changing the milestone from curated parity to full active-catalog parity unless the evidence/work naturally supports it

## Delivery order

Work in this order unless a small reorder clearly improves implementation safety:

1. audit the current curated parity fixture set and define the minimum broadened corpus
2. add curated fixtures covering at least:
   - keywords/modules
   - punctuation/operators
   - numbers
   - comments/whitespace
   - strings/heredocs
   - interpolation
   - selected lexer-owned negative cases if the current harness can support them cleanly
3. change the self-hosted lexer output path so it emits structured JSON matching the reference token dump schema:
   - `kind`
   - `lexeme`
   - `span_start`
   - `span_end`
4. simplify `tests/common/self_hosted_lexer_parity.rs` so it parses JSON from both sides instead of scraping Tonic textual map output
5. split `examples/apps/self_hosted_lexer/src/compiler/lexer.tn` into smaller implementation files with coherent responsibilities
6. update docs/status wording to match the strengthened evidence and no more

## Acceptance criteria

The loop is complete only when all of the following are true:

1. The curated parity corpus contains multiple meaningful fixtures, not just a single happy-path file.
2. The curated corpus covers the main milestone areas:
   - keywords/modules
   - operators/punctuation
   - numeric literals
   - comments/whitespace
   - strings/heredocs
   - interpolation
3. The self-hosted lexer emits structured output directly, rather than requiring the Rust harness to scrape Tonic map/list text.
4. The parity harness compares structured data from both implementations.
5. `examples/apps/self_hosted_lexer/src/compiler/lexer.tn` is split into smaller files/modules so the implementation is materially easier to maintain.
6. All relevant targeted tests pass.
7. Documentation remains honest and does not claim more than the broadened curated evidence supports.

## Verification expectations

Run the smallest sufficient checks during each slice, then run a final focused gate.

Likely relevant surfaces include:

- `cargo test --test check_dump_tokens_json`
- `cargo test --test run_self_hosted_lexer_scaffold_example`
- `cargo test --test self_hosted_lexer_parity`
- any new targeted tests added for JSON output and broadened curated fixtures
- any relevant `cargo test` targets for touched CLI or stdlib surfaces if needed

If you refactor module layout without changing behavior, still prove parity remains green.

## Constraints

- Keep the milestone framing honest: this is still partial self-hosting via a self-hosted lexer milestone.
- Do not widen scope into parser or broader compiler work.
- Do not claim active parity catalog coverage unless you actually wire and verify it.
- Prefer additive fixtures and harness cleanup over speculative architecture churn.
- Follow repo guidance from `AGENTS.md`, including the preference for smaller implementation files.
- Treat dead code and warnings as blockers for completion.
- Commit when tests pass.

## Definition of done

Emit `LOOP_COMPLETE` only when:

- the curated lexer parity corpus is meaningfully broader,
- the self-hosted lexer emits structured output directly,
- the parity harness compares structured outputs cleanly,
- the oversized lexer implementation has been split into smaller files,
- the targeted verification set is green,
- and the docs describe the milestone accurately.
