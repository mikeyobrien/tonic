# Review Notes

Review mode:

- Consistency check: **enabled**
- Completeness check: **enabled**
- Analysis scope: full scan (`update_mode=false`)

## 1) Consistency findings

### ✅ Command surface consistency

- `src/main.rs` command handlers match documented command set in generated docs:
  - `run`, `check`, `test`, `fmt`, `compile`, `cache`, `verify`, `deps`.

### ✅ Native gate/release wiring consistency

- `docs/release-checklist.md` and `scripts/release-alpha-readiness.sh` are aligned:
  - readiness requires clean tree, changelog heading, native gates, artifact existence.

### ✅ ABI docs/code consistency

- `docs/runtime-abi.md` matches `src/native_abi/mod.rs` core boundary concepts (`TValue`, `TCallContext`, `TCallResult`, ABI version checks, deterministic error statuses).

### ⚠️ Parity status nuance

- `PARITY.md` marks structured raise/rescue module forms as implemented.
- `examples/parity/catalog.toml` marks `structured_raise_rescue_module.tn` as `blocked` for LLVM catalog parity due to backend limitation.
- Interpretation: syntax/interpreter support appears implemented, but backend parity gate still has a known limitation.

### ⚠️ Local coding guideline drift

- Root `AGENTS.md` says smaller implementation files are preferred.
- Multiple files significantly exceed 500 lines (e.g., parser/runtime/backend stubs).
- Not a build break, but noteworthy maintenance drift from stated preference.

## 2) Completeness findings

### Gaps in repository-level onboarding docs

- No root `README.md` found at repo root.
- New contributors and assistants must infer project intent via code/docs/parity files instead of a single entrypoint.

### Gaps in generated/runtime C documentation

- Large generated-runtime behavior exists in `src/c_backend/stubs.rs` but lacks focused, up-to-date design docs in `docs/` tied directly to source sections.

### Benchmark/contract docs are stronger than general architecture docs

- Operational/release policy docs are detailed.
- Core architecture rationale is more distributed and less centralized in source docs.

## 3) Language support limitation gaps (explicit)

These are documentation gaps specifically caused by language/tooling support limitations in the analysis process:

1. **Tonic language semantic coverage is inferred from tests/examples, not from a standalone formal spec.**
   - Impact: nuanced semantic edge cases may be under-documented unless traced through parser/lowering/runtime code directly.

2. **Nix/devenv/CI ecosystem files were inventory-scanned but not deeply semantically modeled.**
   - Impact: environment/bootstrap behavior may need manual verification for infra-focused tasks.

3. **Generated C and LLVM execution behavior is represented by emitted-text backends, not a separately versioned ABI spec for every helper path.**
   - Impact: some backend-specific behavior still requires source-level inspection for certainty.

## 4) Recommendations

## High priority

1. Add a concise root `README.md` linking to:
   - project purpose,
   - command quickstart,
   - architecture docs,
   - release/gate scripts.

2. Add a short backend parity status matrix:
   - interpreter support vs native/LLVM parity support per feature.

## Medium priority

3. Add targeted docs for `c_backend/stubs.rs` sections (memory/runtime helper mapping).
4. Introduce a rolling “large file refactor queue” for files >500 LOC (parser, runtime, backend stubs/codegen).

## Low priority

5. Normalize doc cross-links between `PARITY.md`, `docs/native-runtime.md`, and backend parity scripts.
