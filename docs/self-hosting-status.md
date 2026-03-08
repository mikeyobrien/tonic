# Self-Hosting Status

_Last updated: 2026-03-08_

## Current claim

Tonic currently has **partial self-hosting**, specifically a **parity-verified self-hosted lexer** milestone.

That claim is intentionally narrow:

- the self-hosted compiler subsystem is the lexer, not the parser, resolver, typechecker, or backend
- the self-hosted lexer lives at `examples/apps/self_hosted_lexer`
- source text still enters through the host-backed `System.read_text/1` boundary
- the only new kernel support added for this milestone is the narrow traversal primitive `String.to_charlist/1`
- the Rust reference lexer now emits a structured token dump via `tonic check <path> --dump-tokens --format json`
- the self-hosted lexer emits one structured `self_hosted_lexer.tokens` JSON event via `TONIC_SYSTEM_LOG_PATH`, so the harness compares structured token arrays on both sides
- the parity harness compares exact `kind`, `lexeme`, `span_start`, and `span_end`
- mismatch triage artifacts are written by `tests/self_hosted_lexer_parity.rs`
- the current hard gate is the curated eight-fixture lexer corpus under `tests/fixtures/self_hosted_lexer_parity/`, covering keywords/modules, punctuation/operators, numbers/comments/whitespace, strings/heredocs, and interpolation
- this milestone is currently project-mode only

## What is not being claimed

This milestone is **not**:

- full self-hosting
- a Rust-free bootstrap pipeline
- a claim that the active `examples/parity/catalog.toml` corpus is already the self-hosted lexer gate
- a claim that Tonic no longer relies on the host/runtime substrate
- a claim that distribution/install no longer depends on external native toolchains

## Current verification surfaces

The current milestone is evidenced by targeted verification over the touched surfaces:

- `cargo test --bin tonic str_to_charlist`
- `cargo test --test run_lazy_stdlib_loading_smoke run_trace_supports_string_to_charlist_in_project_mode`
- `cargo test --test runtime_llvm_string_stdlib_smoke compiled_runtime_supports_string_stdlib_frontmatter_helper_set_on_literals`
- `cargo test --test check_dump_tokens_json`
- `cargo test --test run_self_hosted_lexer_scaffold_example`
- `cargo test --test self_hosted_lexer_parity`

## Why the gate is curated-only today

The honest milestone gate is still the curated lexer corpus, not the full active parity catalog.

That is deliberate for now:

- the parity harness is currently wired to the curated fixture set in `tests/common/self_hosted_lexer_parity.rs`
- that curated set is now broad enough to justify the narrow lexer milestone claim, but it is still not the same thing as catalog-wide parity coverage
- broadening the gate to the active parity catalog should happen only after an evidence-backed sweep is added and kept green
- until that broader sweep exists, calling the milestone curated-only is more accurate than implying catalog-wide self-hosted parity

## Next strengthening step

The next credibility upgrade is straightforward but separate: broaden the self-hosted lexer parity harness from the curated corpus to an evidence-backed active parity corpus sweep, then keep that broader gate green in the same way the curated gate is enforced today.
