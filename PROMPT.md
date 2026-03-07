# Tonic Runtime/App-Authoring Gap Catalog + Fix Loop

## Objective

Catalog the real application-authoring gaps in Tonic that were surfaced by the `tonic-sitegen-stress` repo, then fix the highest-confidence runtime/stdlib/backend issues directly in the main `tonic` repo.

This loop should produce:

1. a clear gap catalog in the main repo,
2. regression tests/reproducers for each confirmed gap,
3. fixes for the highest-confidence issues,
4. an updated capability picture for building real applications in Tonic.

## External evidence source

Use `/home/mobrienv/projects/tonic-sitegen-stress` as the source of reproducer evidence.

Important evidence currently lives in:

- `src/sitegen_fs.tn`
- `src/sitegen_string.tn`
- `src/sitegen_string_probe.tn`
- `src/sitegen_text_ingestion_probe.tn`
- `src/sitegen_frontmatter_byte_probe.tn`
- `test/verify/fs_discovery_smoke.sh`
- `test/verify/string_probe_smoke.sh`
- `test/verify/text_ingestion_probe.sh`

Do not blindly trust the stress repo conclusions. Reproduce issues in `tonic` itself with minimal tests.

## Problem statement

The stress repo exposed a pattern where Tonic advertises some stdlib/runtime capabilities that are incomplete, missing, or non-parity between interpreter and native execution.

The likely high-priority gaps include:

- `String.*` host functions appear defined but not registered in the active host registry
- `Path.*` host functions appear defined but not registered in the active host registry
- native compiled runtime appears to lack `System.read_text/1`
- native compiled runtime appears to lack `System.read_stdin/0`
- app-authoring filesystem traversal primitives are weak or absent
- stdlib injection in `manifest.rs` may be ahead of actual runtime/backend support

## Scope

### In scope

- adding a gap catalog document in the main repo
- adding minimal failing tests/reproducers for confirmed issues
- fixing confirmed interpreter/runtime/backend mismatches
- fixing stdlib registration issues
- fixing native parity gaps for already-advertised `System.*` APIs
- documenting what is and is not supported for app authors

### Out of scope unless required by the fixes

- building the whole static site generator in this repo
- broad new language design work
- speculative stdlib expansion beyond what the repo already advertises
- OTP/BEAM work

## Delivery order

Work in this order unless evidence strongly suggests a better sequence:

1. create a `docs/app-authoring-gaps.md` catalog
2. add minimal repro tests for the highest-confidence gaps
3. fix `String.*` host registration if confirmed
4. fix `Path.*` host registration if confirmed
5. fix native `System.read_text/1` parity if confirmed
6. fix native `System.read_stdin/0` parity if confirmed
7. update docs/capability notes based on the final result

## Acceptance criteria

The loop is complete only when all of the following are true:

1. The repo contains a clear catalog document of confirmed gaps, status, evidence, and fix status.
2. Every confirmed runtime gap addressed by the loop has a regression test or minimal reproducer in the main repo.
3. If `String.*` registration is missing, it is fixed and covered.
4. If `Path.*` registration is missing, it is fixed and covered.
5. If native `System.read_text/1` parity is missing, it is fixed and covered.
6. If native `System.read_stdin/0` parity is missing, it is fixed and covered.
7. The final documentation distinguishes clearly between:
   - interpreter support
   - native support
   - documented but unsupported features
8. No speculative fixes are landed without tests.

## Verification expectations

At minimum, run the smallest sufficient checks for each slice.

Before final completion, run a final gate that includes all relevant targeted tests for the changed runtime/interop/backend areas. Prefer targeted `cargo test` invocations first, then broader checks if the touched area justifies them.

Examples of likely relevant surfaces:

- `cargo test interop`
- `cargo test system`
- `cargo test compile_aot_artifacts_cli`
- `cargo test cli_contract`
- targeted tests added by this loop

If broad repo-wide gates are required by the changed surface, run them before completion.

## Constraints

- Do not hand-wave with documentation only if the bug is clearly fixable.
- Do not land runtime-facing behavior without regression coverage.
- Do not assume the stress repo is correct; reproduce each claim in `tonic`.
- Prefer fixing the already-advertised contract over inventing new APIs.
- Keep changes small and reviewable.

## Definition of done

Emit `LOOP_COMPLETE` only when the catalog exists, confirmed high-priority gaps are either fixed or explicitly documented as still-blocked with repro evidence, and the required regression tests pass.
