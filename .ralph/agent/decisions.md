# Decision Journal

Template:
- Decision
- Chosen Option
- Confidence (0-100)
- Alternatives Considered
- Reasoning
- Reversibility
- Timestamp (UTC ISO 8601)

## DEC-001
- **Decision:** How to express "scenario IDs" in the Step 1.5 RED test before the feature parser exists.
- **Chosen Option:** Treat each `Scenario:` title as the scenario ID in the failing integration test and assert those IDs plus execution tags appear in `tonic verify run` output.
- **Confidence (0-100):** 68
- **Alternatives Considered:**
  - Require an explicit ID tag format (e.g., `@id:<value>`) in the fixture.
  - Add unit tests against a new parser API that does not exist yet.
- **Reasoning:** The plan requires a red test for tags + scenario IDs, but the ID format is not yet specified. Using scenario titles as IDs is the safest narrow default and keeps the test implementation-ready for Step 1.6 without introducing speculative schema.
- **Reversibility:** High — the test can be updated to a different ID convention once parser/API contracts are formalized.
- **Timestamp (UTC ISO 8601):** 2026-02-20T23:33:38Z

## DEC-002
- **Decision:** Which token stream contract to lock in the first Step 2 RED golden test before lexer internals exist.
- **Chosen Option:** Use a simple line-based textual token format (`TOKEN(value)` or bare delimiter names) for `tonic check ... --dump-tokens` and assert an exact golden stream ending with `EOF`.
- **Confidence (0-100):** 72
- **Alternatives Considered:**
  - Assert only partial substrings (less strict than a golden contract).
  - Choose JSON token output immediately.
- **Reasoning:** Step 2.1 explicitly calls for a golden token test. A line-oriented stream is the narrowest deterministic contract that can drive implementation without committing to a heavier schema too early.
- **Reversibility:** High — output formatting can be migrated later with test updates once a stable diagnostic/reporting format is chosen.
- **Timestamp (UTC ISO 8601):** 2026-02-20T23:46:41Z

## DEC-003
- **Decision:** How to name newly required lexer token labels for Step 2.3 (`|>`, `->`, and `:atom`) in the RED golden test before implementation exists.
- **Chosen Option:** Assert `PIPE_GT` for `|>`, `ARROW` for `->`, and `ATOM(<name>)` for atom literals.
- **Confidence (0-100):** 67
- **Alternatives Considered:**
  - Emit raw operator lexemes as labels (e.g., `|>` / `->`) without symbolic names.
  - Tokenize `:` separately from identifier (`COLON` + `IDENT`).
- **Reasoning:** Symbolic labels keep the golden stream readable and align with existing uppercase token naming (`LPAREN`, `PLUS`) while treating atoms as first-class lexical units needed by upcoming parser work.
- **Reversibility:** High — token names are localized to dump-label formatting and can be migrated with coordinated test updates.
- **Timestamp (UTC ISO 8601):** 2026-02-20T23:51:49Z

## DEC-004
- **Decision:** What AST dump contract to lock for Step 3.1 before parser implementation exists.
- **Chosen Option:** Add a RED integration golden that expects `tonic check <file> --dump-ast` to emit a single-line JSON AST with `modules[]`, each module’s `functions[]`, and minimal expression bodies (`int`, `call`).
- **Confidence (0-100):** 64
- **Alternatives Considered:**
  - Use a custom indentation/tree text format for AST output.
  - Assert only partial substrings instead of an exact golden contract.
- **Reasoning:** A compact JSON contract is deterministic, machine-checkable, and consistent with existing JSON reporting in `verify`, while still keeping the parser scope narrow for Step 3.2.
- **Reversibility:** High — output schema is isolated to dump formatting and can evolve with coordinated test updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:00:56Z

## DEC-005
- **Decision:** How to represent arithmetic precedence in the Step 3.3 RED AST contract before binary-expression parsing exists.
- **Chosen Option:** Expect `+` expressions to serialize as `{"kind":"binary","op":"plus","left":...,"right":...}` and assert a nested-call fixture where call nodes bind tighter than `+`.
- **Confidence (0-100):** 66
- **Alternatives Considered:**
  - Introduce a dedicated `add` expression variant without an `op` field.
  - Assert parse success only, without pinning AST shape.
- **Reasoning:** A generic binary node keeps the contract extensible for future operators and gives Step 3.4 an unambiguous precedence target while staying focused on one operator in v0.
- **Reversibility:** High — schema updates are localized to AST serialization/tests and can be migrated with fixture changes.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:07:08Z

## DEC-006
- **Decision:** How to add stable AST node IDs in Step 3.5 without breaking existing `--dump-ast` JSON contracts.
- **Chosen Option:** Add deterministic parser-side `NodeId` allocation with constructors for `Module`, `Function`, and `Expr`, store IDs on nodes, and mark ID fields `#[serde(skip_serializing)]` so externally observed AST JSON remains unchanged.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Include IDs in serialized AST output and update all integration goldens immediately.
  - Keep IDs in a separate side-table detached from AST nodes.
- **Reasoning:** Embedding IDs directly in nodes keeps future resolver/type phases simple while preserving current CLI contracts. Skipping serialization avoids scope creep into Step 3 output schema.
- **Reversibility:** High — IDs can be exposed later by removing serde skip attributes or moved into a side-table if needed.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:15:00Z

## DEC-007
- **Decision:** What AST JSON contract to lock for Step 4.1 pipe-chain parsing before parser support exists.
- **Chosen Option:** Add a RED integration golden that expects multi-stage `|>` chains to serialize as nested `{"kind":"pipe","left":...,"right":...}` expressions (left-associative) with existing call/int child node shapes.
- **Confidence (0-100):** 65
- **Alternatives Considered:**
  - Represent a pipeline as a `binary` node (`op: pipe`) alongside arithmetic operators.
  - Use a single `pipe` node with a `stages[]` array.
- **Reasoning:** A dedicated `pipe` expression is explicit for later lowering and keeps the contract narrow for Step 4.2 while avoiding an immediate broader binary-operator schema change.
- **Reversibility:** High — output schema is localized to AST serialization/tests and can be migrated with coordinated updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:17:14Z

## DEC-008
- **Decision:** What `case` + pattern AST schema to lock in Step 4.3 RED before parser/pattern support exists.
- **Chosen Option:** Add a RED integration golden expecting a `{"kind":"case"}` body with `subject` expression plus ordered `branches[]`, where patterns use explicit variants (`tuple.items`, `list.items`, `map.entries`, `atom`, `bind`).
- **Confidence (0-100):** 63
- **Alternatives Considered:**
  - Delay schema commitments and assert only command success/failure.
  - Represent patterns as untyped nested token arrays to defer structural choices.
- **Reasoning:** A concrete JSON contract gives Step 4.4 an unambiguous target and keeps the scope narrow to parsing structure only (no type/exhaustiveness semantics yet).
- **Reversibility:** High — this is isolated to AST serialization/tests and can evolve with coordinated fixture updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:21:38Z

## DEC-009
- **Decision:** How to normalize branch representation in Step 4.5 without breaking existing `--dump-ast` case JSON contracts.
- **Chosen Option:** Replace `CaseBranch` with a generic `Branch<Head>` shape (`head` + `body`) plus a `BranchHead` serialization trait so case branches continue to serialize as `{ "pattern": ..., "body": ... }` while parser internals use normalized accessors.
- **Confidence (0-100):** 71
- **Alternatives Considered:**
  - Keep `CaseBranch` as-is and defer normalization to Step 5+.
  - Rename serialized field to `head` and update all case AST fixtures immediately.
- **Reasoning:** Generic branch nodes make future `case`/`cond` typing work simpler and reduce parser coupling, while trait-based serialization preserves every existing AST golden fixture.
- **Reversibility:** High — serialization names and branch wrappers can be adjusted later without touching parser control flow.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:29:37Z

## DEC-010
- **Decision:** What deterministic resolver failure contract to lock in Step 5.1 before any resolver implementation exists.
- **Chosen Option:** Add a RED integration test that expects `tonic check <file>` to fail with `error: [E1001] undefined symbol '<name>' in <Module>.<function>` when a function call target is unresolved in local scope.
- **Confidence (0-100):** 69
- **Alternatives Considered:**
  - Use span-only diagnostics without an error code.
  - Emit a JSON diagnostic payload from `tonic check` instead of stderr text.
- **Reasoning:** Step 5 explicitly requires a deterministic error code. A simple textual code contract keeps scope narrow and works with existing CLI diagnostic plumbing while giving Step 5.2 a concrete behavior target.
- **Reversibility:** High — code naming and message shape are localized to resolver diagnostics and can be remapped later if structured diagnostics are introduced.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:33:00Z

## DEC-011
- **Decision:** What counts as "local scope" for the Step 5.2 baseline resolver.
- **Chosen Option:** Resolve `Expr::Call` targets against a per-module function symbol table only, and run this resolver in default `tonic check` mode (no dump flags); undefined calls emit `[E1001]`.
- **Confidence (0-100):** 77
- **Alternatives Considered:**
  - Keep `tonic check` as a no-op skeleton and only add a resolver library API.
  - Treat every call as valid until imports/module graph support lands in Step 5.4.
  - Add prelude/builtin symbol exceptions now.
- **Reasoning:** The current RED contract is scoped to deterministic undefined-symbol diagnostics in a single module. Restricting baseline resolution to module-local function names keeps implementation narrow, matches existing AST capabilities, and avoids speculative import/builtin behavior before Step 5.3/5.4.
- **Reversibility:** High — symbol lookup rules can be expanded later (imports, qualified names, builtins) behind the same resolver entrypoint.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:35:20Z

## DEC-012
- **Decision:** What cross-module contract to lock in the Step 5.3 RED test before import syntax and module-graph loading are implemented.
- **Chosen Option:** Add a failing integration test that expects `tonic check` to accept a module-qualified call (`Math.helper()`) across two modules in one source file.
- **Confidence (0-100):** 70
- **Alternatives Considered:**
  - Use an explicit `import` fixture (syntax not yet defined/parsable).
  - Require directory-based multi-file loading in this RED test.
- **Reasoning:** The parser currently supports single-expression function bodies and no import declarations. A module-qualified call sets an unambiguous target for Step 5.4 (token/parsing + resolver lookup) while keeping this RED slice minimal and test-driven.
- **Reversibility:** High — the contract can be extended to explicit imports and directory module graphs in follow-up slices.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:38:38Z

## DEC-013
- **Decision:** How to represent module-qualified call expressions while adding Step 5.4 cross-module resolution.
- **Chosen Option:** Keep `Expr::Call { callee: String, ... }` and encode qualified calls as dotted strings (for example `"Math.helper"`), then resolve module/function targets via a resolver-side module graph.
- **Confidence (0-100):** 76
- **Alternatives Considered:**
  - Add a dedicated `Expr::QualifiedCall { module, function, args }` AST variant.
  - Introduce a parser-level call-target enum (`Local` vs `Qualified`) and change AST JSON contracts.
- **Reasoning:** This keeps parser and AST changes minimal, preserves all existing `--dump-ast` fixture contracts, and still enables module graph lookup for cross-module calls in the current Step 5 scope.
- **Reversibility:** High — if import syntax or richer call metadata is needed later, the resolver can be refactored to a structured call target without changing current CLI behavior.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:42:35Z

## DEC-014
- **Decision:** How to centralize resolver diagnostics in Step 5.5 without changing existing CLI error contracts.
- **Chosen Option:** Extract resolver error code/message construction into a dedicated `resolver_diag` module (`ResolverDiagnosticCode` + `ResolverError`) and update `resolver` to consume shared constructors while keeping `[E1001] ...` output unchanged.
- **Confidence (0-100):** 75
- **Alternatives Considered:**
  - Keep diagnostics inline in `resolver.rs` and only add comments/constants.
  - Introduce a more generic compiler-wide diagnostic framework now.
- **Reasoning:** A focused module extraction satisfies Step 5.5 centralization with minimal scope, avoids speculative cross-phase architecture, and keeps all existing tests/CLI contracts stable.
- **Reversibility:** High — the diagnostics module can later be expanded into a broader diagnostic catalog without changing resolver traversal logic.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:46:30Z

## DEC-015
- **Decision:** What initial type-inference contract to lock for Step 6.1 RED before an inference engine exists.
- **Chosen Option:** Introduce a dedicated `typing::infer_types(&Ast)` unit test that expects deterministic function signatures for a polymorphic-like helper (`Demo.helper` => `fn(dynamic) -> int`) and a concrete consumer (`Demo.run` => `fn() -> int`).
- **Confidence (0-100):** 66
- **Alternatives Considered:**
  - Add another CLI integration test for `tonic check` success/failure only (would not force type inference).
  - Add a new CLI dump flag (`--dump-types`) before the typing core exists.
- **Reasoning:** CLI pass/fail behavior is currently resolver-only, so another integration test would not provide type-inference backpressure. A focused unit-level contract is the narrowest way to force real inference behavior while reusing the existing AST pipeline.
- **Reversibility:** High — signature string format and API shape can be revised once typed-module data structures stabilize.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:51:14Z

## DEC-016
- **Decision:** Whether Step 6.2 should constrain call-site argument types into callee parameter variables while implementing the first unification pass.
- **Chosen Option:** Infer return types via expression constraints and unification, but do not yet flow call-site argument constraints back into callee parameters; unresolved parameters finalize to `dynamic`.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Fully unify call-site argument types with callee parameter variables now.
  - Hardcode every parameter as `dynamic` and skip type-variable/unification scaffolding.
- **Reasoning:** The locked RED contract expects `Demo.helper` to remain `fn(dynamic) -> int` even with concrete int call sites. Deferring argument-to-parameter unification keeps this slice narrow, preserves the RED expectation, and still introduces a real constraint solver for return and operator typing.
- **Reversibility:** High — call-site parameter constraints can be added in a follow-up slice once explicit mismatch policy and dynamic-boundary rules are locked.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:55:36Z

## DEC-017
- **Decision:** What concrete mismatch fixture and span contract to lock for Step 6.3 RED before mismatch diagnostics exist.
- **Chosen Option:** Add a failing typing unit test where `unknown()` returns `dynamic` via an empty `case` expression and is used in `unknown() + 1`; assert inference fails with `[E2001] type mismatch: expected int, found dynamic at offset 123`.
- **Confidence (0-100):** 68
- **Alternatives Considered:**
  - Assert only that inference returns `Err` without pinning code/message/offset.
  - Use a CLI integration test (current `tonic check` path is resolver-only, so mismatch behavior would not be exercised).
- **Reasoning:** This is the narrowest deterministic RED contract that directly pressures Step 6.4 to implement both coercion rejection and span-aware diagnostics without changing CLI routing yet.
- **Reversibility:** High — the exact diagnostic text/offset contract is localized to one unit test and can be adjusted with coordinated updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T00:58:55Z

## DEC-018
- **Decision:** How to provide span offsets for Step 6.4 mismatch diagnostics without changing existing AST JSON fixtures.
- **Chosen Option:** Add parser-only expression `offset` metadata (skipped during serialization) to every `Expr` variant, then use those offsets in typing unification errors while rejecting implicit `dynamic`↔`int` coercions.
- **Confidence (0-100):** 78
- **Alternatives Considered:**
  - Hardcode the RED-test offset in typing diagnostics.
  - Keep AST unchanged and infer offsets indirectly from node IDs.
  - Add fully serialized spans to AST output and update all dump-ast contracts.
- **Reasoning:** Hidden per-expression offsets give deterministic span-aware diagnostics with minimal blast radius. This preserves all existing `--dump-ast` goldens and avoids brittle special-casing in the type checker.
- **Reversibility:** High — offset metadata can be replaced with richer span structs later without affecting external AST JSON contracts.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:03:41Z

## DEC-019
- **Decision:** Which explicit `dynamic` annotation syntax/policy to lock in Step 6.5 RED before implementation exists.
- **Chosen Option:** Treat `dynamic` as an explicit parameter annotation marker (`def helper(dynamic value)`) and lock a deterministic parser rejection for function return-position annotation (`def run() -> dynamic do`) with message `dynamic annotation is only allowed on parameters at offset 30`.
- **Confidence (0-100):** 61
- **Alternatives Considered:**
  - Use expression-level escape hatch syntax (for example `dynamic(expr)`) and defer parameter annotations.
  - Introduce colon-based type annotations (`value: dynamic`) despite current lexer/parser shape.
  - Assert only generic parse failure without a specific policy diagnostic.
- **Reasoning:** Parameter-position annotation is the narrowest additive extension compatible with the current grammar while still enforcing a concrete “allowed vs disallowed” policy. Locking a deterministic rejection message now creates clear backpressure for the upcoming GREEN implementation.
- **Reversibility:** Medium-High — syntax can be migrated later, but this contract intentionally pins one concrete policy for the next slice.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:09:43Z

## DEC-020
- **Decision:** How to implement explicit `dynamic` parameter boundaries without breaking existing AST JSON fixtures.
- **Chosen Option:** Introduce parser-side `Parameter` nodes with a hidden `ParameterAnnotation` (`Inferred` vs `Dynamic`), keep `params` serialized as plain strings via custom `Serialize`, reject `-> dynamic` after function heads with a policy-specific parser diagnostic, and seed typing parameter constraints from the annotation.
- **Confidence (0-100):** 73
- **Alternatives Considered:**
  - Keep `params: Vec<String>` and ignore annotation semantics after parsing.
  - Change AST JSON schema to expose parameter objects and update all dump fixtures.
  - Handle annotation policy only in typing, leaving parser grammar unchanged.
- **Reasoning:** Hidden parameter metadata gives us real explicit-boundary semantics for typing while preserving every existing `--dump-ast` contract. Parser-side rejection of `-> dynamic` enforces the policy at the right phase with deterministic offsets.
- **Reversibility:** High — parameter metadata can be surfaced or refactored later without changing current JSON output.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:13:10Z
