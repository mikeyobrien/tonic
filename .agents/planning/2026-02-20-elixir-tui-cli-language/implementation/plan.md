# Implementation Plan (Micro-TDD) — Language Core v0

Convert the design into a series of implementation steps that build each component in a test-driven, incremental way. Each step below ends in a working, demoable increment and integrates with prior steps.

## Progress Checklist

- [ ] Step 1: Bootstrap workspace + acceptance backpressure harness + BDD foundation
- [ ] Step 2: Lexer for Elixir-inspired token set
- [ ] Step 3: Parser for modules/functions and core expressions
- [ ] Step 4: Parser extensions for pipe + pattern forms
- [ ] Step 5: Resolver + module graph foundation
- [ ] Step 6: Static type inference core + explicit `dynamic` escape hatch
- [ ] Step 7: Result-first semantics + `?` propagation + exhaustiveness checks
- [ ] Step 8: Lowering typed AST to executable core IR
- [ ] Step 9: Interpreter runtime + `tonic run` end-to-end execution
- [ ] Step 10: Core data model + protocols + Enum-style stdlib subset
- [ ] Step 11: Project manifest/module loader + lazy stdlib loading
- [ ] Step 12: On-disk compiled cache + invalidation rules
- [ ] Step 13: `check/test/fmt/verify` + BDD dual-run modes + performance gate enforcement

---

## Step 1: Bootstrap workspace + acceptance backpressure harness + BDD foundation

**Objective**
Create the Rust workspace, CLI skeleton, and acceptance verification scaffolding with a BDD source-of-truth model so every future slice is gate-driven.

**Implementation guidance**
- Create crates/modules for: `cli`, `frontend`, `resolver`, `typing`, `ir`, `runtime`, `verify`.
- Add `tonic` command skeleton (`run/check/test/fmt/cache/verify`).
- Add `acceptance/<slice-id>.yaml` schema + parser + status reporter.
- Add `acceptance/features/<slice-id>.feature` support and scenario tag parsing (`@auto`, `@agent-manual`, `@human-manual`).

**Micro TDD tasks**
1. **Red:** CLI smoke test expects listed commands in `--help` output.
2. **Green:** implement command skeleton with stable exit codes.
3. **Red:** verification test expects failure when acceptance file missing.
4. **Green:** implement acceptance file loading + explicit error.
5. **Red:** feature parser test validates tags and scenario IDs.
6. **Green:** implement minimal feature parsing and mode selection metadata.
7. **Refactor:** extract shared CLI diagnostics and acceptance parsing utilities.

**Test requirements**
- Unit tests for command routing, acceptance parser, and BDD feature tag parsing.
- Integration test for `tonic verify run step-01 --mode auto`.

**Integration with previous work**
- Establishes backpressure mechanism and BDD acceptance contract required by design.

**Demo**
- `tonic --help` shows all v0 commands.
- `tonic verify run step-01 --mode auto` reads acceptance YAML + linked feature file and outputs pass/fail JSON.

---

## Step 2: Lexer for Elixir-inspired token set

**Objective**
Implement deterministic tokenization for v0 syntax subset.

**Implementation guidance**
- Support identifiers, atoms, numbers, strings, operators, delimiters, `defmodule/def/if/case/cond/fn` keywords, `|>`.
- Preserve source spans for diagnostics.

**Micro TDD tasks**
1. **Red:** token golden test for a minimal module file.
2. **Green:** implement baseline scanner for identifiers/literals.
3. **Red:** operator/keyword token test (`|>`, `->`, `:` atoms, etc.).
4. **Green:** add operator and keyword tokenization.
5. **Refactor:** unify span handling and error token reporting.

**Test requirements**
- Golden token snapshots.
- Invalid token diagnostics test.

**Integration with previous work**
- Feeds parser entrypoint added in Step 1.

**Demo**
- `tonic check examples/lexer_smoke.tn --dump-tokens` prints expected token stream.

---

## Step 3: Parser for modules/functions and core expressions

**Objective**
Parse modules, function declarations, and core expressions into AST.

**Implementation guidance**
- AST support for `defmodule`, `def`, function params, literals, calls, `if`.
- Keep parser deterministic and span-rich.

**Micro TDD tasks**
1. **Red:** parse test for single-module two-function file.
2. **Green:** implement module/function parser path.
3. **Red:** expression parse test for call nesting and precedence.
4. **Green:** implement expression parser with precedence table.
5. **Refactor:** AST node constructors with stable IDs.

**Test requirements**
- AST snapshot tests.
- Parse error recovery tests (single-file).

**Integration with previous work**
- Consumes lexer output from Step 2.

**Demo**
- `tonic check examples/parser_smoke.tn --dump-ast` outputs valid AST.

---

## Step 4: Parser extensions for pipe + pattern forms

**Objective**
Add parsing for pipe chains and pattern forms needed by `case` and function heads.

**Implementation guidance**
- Parse `|>` chain nodes.
- Parse tuple/list/map/wildcard/bind patterns.
- Parse `case`/`cond` branches with pattern heads.

**Micro TDD tasks**
1. **Red:** parse test for multi-stage pipe chain.
2. **Green:** implement pipe AST lowering shape.
3. **Red:** parse test for tuple/list/map patterns in `case`.
4. **Green:** implement pattern parser variants.
5. **Refactor:** normalize branch representation for future type checking.

**Test requirements**
- Pattern syntax fixtures.
- Parser diagnostics for malformed pattern branches.

**Integration with previous work**
- Extends Step 3 AST model without breaking existing fixtures.

**Demo**
- `tonic check examples/pipes_patterns.tn --dump-ast` succeeds.

---

## Step 5: Resolver + module graph foundation

**Objective**
Resolve symbols and module references with clear diagnostics.

**Implementation guidance**
- Build scoped symbol tables.
- Resolve local names, module-qualified names, and imports.
- Create initial module graph representation.

**Micro TDD tasks**
1. **Red:** undefined symbol test should emit deterministic error code.
2. **Green:** implement local scope symbol resolution.
3. **Red:** import resolution test across two modules.
4. **Green:** implement module reference resolution.
5. **Refactor:** centralize resolver diagnostics and codes.

**Test requirements**
- Resolver unit tests (shadowing, ambiguity, missing imports).
- Two-module integration fixture.

**Integration with previous work**
- Takes AST from Steps 3–4 and prepares typed analysis input.

**Demo**
- `tonic check examples/multi_module_basic/` reports either resolved graph or actionable resolver errors.

---

## Step 6: Static type inference core + explicit `dynamic` escape hatch

**Objective**
Implement mostly strict static inference with explicit `dynamic` boundaries.

**Implementation guidance**
- Infer primitive/composite/function types.
- Reject implicit unsafe coercions.
- Allow explicit `dynamic` at controlled boundaries.

**Micro TDD tasks**
1. **Red:** inference test for polymorphic-like helper and concrete call sites.
2. **Green:** implement base type constraints + unification.
3. **Red:** mismatch test expects type error with spans.
4. **Green:** add mismatch diagnostics and coercion rejection.
5. **Red/Green:** explicit `dynamic` annotation accepted only where allowed.

**Test requirements**
- Inference suite + type error snapshots.
- Dynamic boundary policy tests.

**Integration with previous work**
- Uses resolved AST from Step 5.

**Demo**
- `tonic check examples/type_inference_smoke.tn` passes; `examples/type_error_smoke.tn` fails with structured diagnostics.

---

## Step 7: Result-first semantics + `?` propagation + exhaustiveness checks

**Objective**
Enforce `ok/err`-first flow and compile-time match exhaustiveness.

**Implementation guidance**
- Add `Result<T,E>` typing rules.
- Implement `?` propagation typing and lowering hooks.
- Add non-exhaustive pattern diagnostics.

**Micro TDD tasks**
1. **Red:** `?` on non-Result expression must fail type checking.
2. **Green:** implement `?` typing rule for `Result`.
3. **Red:** non-exhaustive `case` fixture should fail.
4. **Green:** implement exhaustiveness checker for v0 pattern subset.
5. **Refactor:** harmonize error codes/messages for Result+match failures.

**Test requirements**
- Result propagation tests.
- Exhaustiveness matrix tests.

**Integration with previous work**
- Extends type checker and resolver outputs.

**Demo**
- `tonic check examples/result_flow.tn` passes.
- `tonic check examples/non_exhaustive_case.tn` fails with expected error code.

---

## Step 8: Lowering typed AST to executable core IR

**Objective**
Translate typed AST into compact, executable typed IR with source maps.

**Implementation guidance**
- Define core IR nodes for expressions, branches, calls, pattern checks.
- Preserve source mapping for runtime diagnostics.

**Micro TDD tasks**
1. **Red:** lowering snapshot for simple typed function.
2. **Green:** implement lowering for literals/calls/control flow.
3. **Red:** lowering snapshot for `?` and `case` branches.
4. **Green:** implement lowering for Result/match constructs.
5. **Refactor:** canonicalize IR forms for interpreter simplicity.

**Test requirements**
- IR snapshot tests.
- Source map integrity tests.

**Integration with previous work**
- Consumes typed module from Steps 6–7.

**Demo**
- `tonic check examples/ir_smoke.tn --dump-ir` outputs stable IR.

---

## Step 9: Interpreter runtime + `tonic run` end-to-end execution

**Objective**
Execute core IR deterministically for CLI scripts.

**Implementation guidance**
- Implement call frames, value model, control-flow execution.
- Return `ok/err` semantics through CLI exit behavior.
- Wire `tonic run` to full frontend->runtime path.

**Micro TDD tasks**
1. **Red:** E2E run test for arithmetic/script output.
2. **Green:** implement minimal evaluator loop.
3. **Red:** runtime Result propagation behavior test.
4. **Green:** implement `err` propagation and CLI exit code mapping.
5. **Refactor:** frame/value internals for lower allocation churn.

**Test requirements**
- Runtime unit tests for evaluator ops.
- End-to-end `tonic run` fixtures.

**Integration with previous work**
- Executes IR from Step 8.

**Demo**
- `tonic run examples/hello.tn` prints expected output with stable exit status.

---

## Step 10: Core data model + protocols + Enum-style stdlib subset

**Objective**
Deliver practical v0 language ergonomics: maps/tuples/keywords, protocols, and core collection APIs.

**Implementation guidance**
- Implement runtime representations for tuple/map/list/keyword list.
- Add protocol declaration/implementation/dispatch (v0 subset).
- Implement initial Enum-style functions used by core examples.

**Micro TDD tasks**
1. **Red:** map/tuple/keyword behavior fixture tests.
2. **Green:** implement data structure runtime support.
3. **Red:** protocol dispatch test across two concrete types.
4. **Green:** implement protocol lookup/dispatch tables.
5. **Red/Green:** pipe + Enum fixture for chained transformations.

**Test requirements**
- Stdlib behavior tests.
- Protocol dispatch conformance tests.

**Integration with previous work**
- Extends interpreter semantics from Step 9.

**Demo**
- `tonic run examples/collections_protocols.tn` demonstrates pipelines + protocol dispatch.

---

## Step 11: Project manifest/module loader + lazy stdlib loading

**Objective**
Support project-level execution using `tonic.toml` and module graph loading with lazy optional modules.

**Implementation guidance**
- Parse/validate manifest.
- Build module graph for project-local modules.
- Eager-load core modules; lazy-load optional stdlib modules on first use.

**Micro TDD tasks**
1. **Red:** manifest parse/validation tests.
2. **Green:** implement manifest model and loader.
3. **Red:** multi-module project run fixture test.
4. **Green:** implement module loader and graph resolution.
5. **Red/Green:** lazy-load test ensures optional module not loaded unless referenced.

**Test requirements**
- Manifest unit tests.
- Multi-module integration tests.
- Lazy-loading behavior tests (module-load tracing).

**Integration with previous work**
- Wraps runtime pipeline into project-level UX.

**Demo**
- `tonic run examples/project_smoke` executes multi-module project and logs lazy-load behavior in debug mode.

---

## Step 12: On-disk compiled cache + invalidation rules

**Objective**
Add persistent cache for lowered typed IR to accelerate warm starts.

**Implementation guidance**
- Implement cache key (entry hash + dep hash + runtime version + target + flags).
- Serialize/deserialize IR.
- Invalidate on key mismatch; recover from corruption gracefully.

**Micro TDD tasks**
1. **Red:** cache hit/miss unit tests with synthetic keys.
2. **Green:** implement cache key and storage interface.
3. **Red:** integration test verifies second run uses cache.
4. **Green:** wire cache lookup/store into run pipeline.
5. **Red/Green:** corruption test forces fallback compile path.

**Test requirements**
- Cache unit + integration tests.
- Deterministic invalidation tests.

**Integration with previous work**
- Accelerates Step 11 execution path; no behavior changes when cache misses.

**Demo**
- Running same script twice shows first compile path then cache-hit path; warm run latency improves.

---

## Step 13: `check/test/fmt/verify` + BDD dual-run modes + performance gate enforcement

**Objective**
Complete v0 developer workflow and enforce acceptance/performance backpressure through dual-run BDD verification before completion.

**Implementation guidance**
- Finalize `tonic check`, `tonic test`, `tonic fmt`, `tonic verify`.
- Add `tonic verify` modes:
  - `--mode auto` (run `@auto` scenarios only)
  - `--mode mixed` (run `@auto` + `@agent-manual`)
  - `--mode manual` (run full manual-tagged checks)
- Add benchmark harness and CI thresholds:
  - cold start p50 <= 50 ms
  - warm start p50 <= 10 ms
  - idle RSS <= 30 MB
- Enforce acceptance files + feature links + step evidence requirements in verify flow.

**Micro TDD tasks**
1. **Red:** command integration tests for check/test/fmt paths.
2. **Green:** implement missing command behavior and output contracts.
3. **Red:** BDD mode tests assert tag filtering (`@auto`, `@agent-manual`, `@human-manual`).
4. **Green:** implement dual-run BDD mode execution in verify runner.
5. **Red:** benchmark gate test should fail when threshold exceeded.
6. **Green:** implement threshold enforcement and structured report output.
7. **Red/Green:** verify command must fail if required manual evidence JSON is missing.

**Test requirements**
- CLI integration suite for all commands.
- BDD mode integration tests for auto/mixed/manual.
- Performance benchmark tests in CI profile.
- Acceptance workflow tests for blocked vs accepted statuses.

**Integration with previous work**
- Brings all components together into a complete, shippable v0 language-core toolchain with unified executable/manual acceptance.

**Demo**
- `tonic verify run step-13 --mode auto` runs only automated BDD scenarios.
- `tonic verify run step-13 --mode mixed` requires agent-manual evidence and fails without it.
- Completion succeeds only when tests, benchmarks, and required evidence checks all pass.

---

## Connections
- [[../design/detailed-design.md]]
- [[../idea-honing.md]]
- [[../research/research-plan.md]]
- [[../research/06-runtime-semantics-gap.md]]
- [[../research/07-startup-memory-techniques.md]]
- [[../research/08-toolchain-portability-gap.md]]
- [[../research/09-terminal-portability-gap.md]]
- [[../research/10-practitioner-signals.md]]
