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

## DEC-021
- **Decision:** What deterministic RED diagnostic contract to lock for Step 7.1 before `?` parsing and Result typing exist.
- **Chosen Option:** Add a failing CLI integration test that expects `tonic check` to report `error: [E3001] ? operator requires Result value, found int at offset 74` for `value()?` when `value/0` returns `int`.
- **Confidence (0-100):** 67
- **Alternatives Considered:**
  - Write a unit test against `typing::infer_types` first (requires AST support for `?` that does not exist yet).
  - Assert only generic check failure without pinning a code/message.
- **Reasoning:** An integration contract keeps the scope narrow while forcing the next GREEN slice to wire lexer/parser/type-checking for `?` end-to-end. Pinning code/message now prevents ambiguous failure behavior during implementation.
- **Reversibility:** High — the exact code/message can be revised later with coordinated test updates once a broader diagnostics catalog is finalized.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:18:00Z

## DEC-022
- **Decision:** How to implement Step 7.2 `?` support while keeping existing parser and AST dump contracts stable.
- **Chosen Option:** Add `?` as a postfix expression (`Expr::Question`) with hidden offset metadata, enforce `Result` requirement in typing (`[E3001]` on non-Result), and introduce minimal `ok/err` builtin Result constructors for positive-path inference coverage.
- **Confidence (0-100):** 72
- **Alternatives Considered:**
  - Keep `?` as syntax sugar rejected everywhere until a fuller Result type system lands.
  - Parse `?` but defer all checks to runtime.
  - Add an explicit serialized AST schema change for try-propagation nodes immediately.
- **Reasoning:** This is the narrowest end-to-end implementation that satisfies the locked RED integration contract, preserves current JSON output fixtures, and leaves room for richer Result semantics in later steps.
- **Reversibility:** High — builtin handling and `Expr::Question` lowering behavior can be refined later without changing current CLI diagnostic contracts.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:22:32Z

## DEC-023
- **Decision:** What initial non-exhaustive `case` contract to lock for Step 7.3 RED before exhaustiveness checking exists.
- **Chosen Option:** Add a failing CLI integration test that expects `tonic check` to reject a `case` expression lacking `_` with `error: [E3002] non-exhaustive case expression: missing wildcard branch at offset 37`.
- **Confidence (0-100):** 64
- **Alternatives Considered:**
  - Start with unit tests in `typing.rs` only and defer CLI contract.
  - Require full pattern-coverage analysis (atom/tuple/list/map combinations) in the first RED contract.
  - Assert only generic failure without pinning a code/message.
- **Reasoning:** The plan calls for a non-exhaustive `case` failure, but full v0 coverage analysis is broader than one RED slice. Requiring a wildcard fallback is the narrowest deterministic contract that drives Step 7.4 implementation while preserving current parser/type-check architecture.
- **Reversibility:** High — this baseline contract can expand to richer coverage analysis once additional pattern semantics are implemented.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:27:00Z

## DEC-024
- **Decision:** How to scope Step 7.4 exhaustiveness checking for `case` without overbuilding pattern-coverage analysis.
- **Chosen Option:** Enforce a v0 baseline rule in typing: every `case` must include at least one top-level wildcard (`_`) branch; otherwise emit `[E3002] non-exhaustive case expression: missing wildcard branch` at the `case` expression offset.
- **Confidence (0-100):** 76
- **Alternatives Considered:**
  - Implement full structural pattern coverage (atom/tuple/list/map space) in this slice.
  - Add parser-time rejection instead of typing-time diagnostics.
  - Only fail empty branch lists and allow non-wildcard-only cases.
- **Reasoning:** The locked RED contract is explicitly about missing wildcard coverage. A wildcard-presence rule is deterministic, minimal, and reversible while still enforcing concrete exhaustiveness backpressure in `tonic check`.
- **Reversibility:** High — richer coverage analysis can replace/extend this guard in follow-up slices without breaking current diagnostic plumbing.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:31:25Z

## DEC-025
- **Decision:** How to refactor Step 7.5 Result/match diagnostics without introducing a broad compiler-wide diagnostics framework.
- **Chosen Option:** Extract typing diagnostics into a dedicated `typing::diag` module and route both `[E3001]` (`?` requires Result) and `[E3002]` (non-exhaustive case) through shared constructors, while exposing test-only `code()`/`message()` accessors.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Keep diagnostic code/message constructors inline in `typing.rs` and only add test assertions on rendered strings.
  - Introduce a global diagnostics abstraction spanning lexer/parser/resolver/typing in this slice.
- **Reasoning:** A focused extraction harmonizes Result+match error handling in one place, matches the prior resolver-diagnostics refactor pattern, and preserves existing CLI contracts without over-scoping the iteration.
- **Reversibility:** High — the module can be folded into a future shared diagnostics layer or expanded with richer metadata later.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:37:17Z

## DEC-026
- **Decision:** What initial IR JSON contract to lock for Step 8.1 RED before lowering exists.
- **Chosen Option:** Add a failing integration test for `tonic check <file> --dump-ir` that expects a compact function-level IR snapshot with linear ops (`const_int`, `return`) for `Demo.run/0`.
- **Confidence (0-100):** 66
- **Alternatives Considered:**
  - Assert only that `--dump-ir` succeeds without pinning output schema.
  - Reuse AST JSON shape for `--dump-ir` to minimize implementation effort.
  - Start with unit-only lowering tests before exposing any CLI contract.
- **Reasoning:** Step 8.1 requires a lowering snapshot contract. Locking a minimal but distinct ops-based JSON shape creates clear backpressure for Step 8.2 while avoiding over-scoping into source maps and control-flow structure.
- **Reversibility:** High — the IR schema is still early and can evolve with coordinated snapshot updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:39:00Z

## DEC-027
- **Decision:** How to scope the first Step 8.2 GREEN IR lowering implementation while keeping the path ready for later Result/case lowering.
- **Chosen Option:** Introduce a dedicated `ir` module with a compact op stream (`const_int`, `call`, `add_int`, `return`), wire `tonic check --dump-ir` through resolver+typing before lowering, and fail fast on unsupported expression forms (`question`, `pipe`, `case`) for now.
- **Confidence (0-100):** 77
- **Alternatives Considered:**
  - Lower every current AST variant immediately (including `?` and `case`) before any Step 8.3 RED contract exists.
  - Emit IR directly from parser output without running typing first.
  - Reuse AST JSON output for `--dump-ir` to avoid introducing a new lowering layer.
- **Reasoning:** The active GREEN contract only requires deterministic IR output for a simple typed function and explicitly names literals/calls. A focused lowering module plus CLI wiring keeps the slice minimal, preserves test determinism, and avoids overbuilding ahead of the next RED steps.
- **Reversibility:** High — op names and unsupported-form handling are localized to `src/ir.rs` and can be extended once Step 8.3 contracts are locked.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:42:51Z

## DEC-028
- **Decision:** What Step 8.3 RED snapshot shape to lock for IR lowering that combines `?` propagation and `case` branches.
- **Chosen Option:** Add a failing integration test for `tonic check --dump-ir` that expects a compact ops stream containing `question` and `case` ops, with explicit branch payloads (`pattern` + branch `ops`) for an atom branch and wildcard fallback.
- **Confidence (0-100):** 69
- **Alternatives Considered:**
  - Assert only command failure/success without pinning IR JSON details.
  - Add unit-only lowering tests in `src/ir.rs` and defer CLI snapshot coverage.
  - Keep rejecting `question`/`case` until a larger runtime design is finalized.
- **Reasoning:** Step 8.3 explicitly asks for a lowering snapshot on `?` + `case`. Locking the CLI JSON contract now gives Step 8.4 clear implementation backpressure and keeps scope focused on IR shape rather than runtime semantics.
- **Reversibility:** High — op names and branch payload schema are early-stage and can evolve with coordinated snapshot updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:46:25Z

## DEC-029
- **Decision:** How to implement Step 8.4 lowering for `Expr::Question` and `Expr::Case` while preserving the locked IR snapshot contract.
- **Chosen Option:** Lower `?` into a dedicated `{"op":"question"}` instruction emitted after lowering the operand, lower `case` into a single `{"op":"case","branches":[...]}` instruction with per-branch `{pattern, ops}` payloads, and keep `ok/err` constructor calls unqualified (`ok`, `err`) in IR output.
- **Confidence (0-100):** 75
- **Alternatives Considered:**
  - Continue qualifying all unqualified call targets (`Demo.ok`) and adjust snapshots.
  - Desugar `case` into a flattened jump-like op sequence instead of structured branch payloads.
  - Keep rejecting `case`/`question` in lowering until a fuller runtime control-flow design lands.
- **Reasoning:** The Step 8.3 RED test already locked an explicit JSON shape. Emitting focused `question` + structured `case` ops is the narrowest GREEN implementation that satisfies the contract and keeps future interpreter work straightforward.
- **Reversibility:** High — op/pattern schema is localized to `src/ir.rs` and can be canonicalized in Step 8.5 with coordinated snapshot updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:49:52Z

## DEC-030
- **Decision:** How to canonicalize IR call forms in Step 8.5 without overhauling the whole lowering schema.
- **Chosen Option:** Refactor `IrOp::Call` to carry a structured `callee` target (`{"kind":"function","name":...}` or `{"kind":"builtin","name":...}`) and keep existing linear op ordering unchanged.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Keep string callees and defer canonicalization to the Step 9 interpreter.
  - Introduce a larger control-flow IR redesign (basic blocks/jumps) in this slice.
  - Flatten builtin/function differences into naming conventions only.
- **Reasoning:** Step 8.5 asks for canonical IR forms that simplify interpreter work. Explicit call-target kinds remove string parsing heuristics in runtime dispatch while staying narrowly scoped to one op payload and preserving current lowering behavior.
- **Reversibility:** High — the enum can be extended (for extern/intrinsic targets) or flattened later with coordinated snapshot updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:54:12Z

## DEC-031
- **Decision:** What source-map contract to lock for Step 8 after canonical IR call-target refactoring.
- **Chosen Option:** Add a RED integration test requiring `tonic check ... --dump-ir` to include per-op `offset` fields (at least for `const_int` and `return`) so IR snapshots carry source mapping metadata.
- **Confidence (0-100):** 71
- **Alternatives Considered:**
  - Defer source-map requirements to Step 9 runtime diagnostics.
  - Add unit-only lowering assertions without pinning CLI JSON output.
  - Introduce a separate `source_map` section instead of op-local offsets.
- **Reasoning:** Step 8 explicitly calls for source-map integrity tests. Locking op-local offsets in the dump snapshot is the narrowest deterministic contract that pressures lowering changes while keeping IR shape simple.
- **Reversibility:** High — offsets can later move into richer span metadata or a side-table with coordinated snapshot updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T01:56:30Z

## DEC-032
- **Decision:** How broadly to apply source offsets while implementing Step 8.7 IR source-map metadata.
- **Chosen Option:** Add an `offset` field to every `IrOp` variant and emit the originating AST expression offset for each lowered op (including branch-local ops and synthetic `return` using the function body offset), then update existing IR snapshots to the new schema.
- **Confidence (0-100):** 78
- **Alternatives Considered:**
  - Add offsets only to `const_int` and `return` to satisfy the minimal new integration test.
  - Emit a separate top-level source-map table instead of op-local metadata.
- **Reasoning:** Although the RED contract only pins `const_int`/`return`, op-local offsets on all instructions keep IR diagnostics uniform and avoid a follow-up schema migration as soon as runtime error mapping expands beyond literals.
- **Reversibility:** High — offsets can later move to richer span structs or a dedicated source-map section with coordinated snapshot updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:00:30Z

## DEC-033
- **Decision:** What initial runtime entrypoint/output contract to lock in Step 9.1 RED.
- **Chosen Option:** Add an integration test expecting `tonic run <path>` to execute a `run()` function from the script module and print the evaluated arithmetic result (`3\n`) to stdout.
- **Confidence (0-100):** 72
- **Alternatives Considered:**
  - Require an explicit entrypoint argument (`Module.function`) before implementing `run` semantics.
  - Execute the first declared function implicitly without naming convention.
  - Delay output assertions and only check that `run` no longer prints the skeleton placeholder.
- **Reasoning:** The plan calls for an end-to-end arithmetic/script output test first. A `run()` convention is the narrowest deterministic contract compatible with the current module/function-only parser and gives the GREEN step a clear target.
- **Reversibility:** High — CLI entrypoint selection can be expanded later (explicit flags, manifest defaults) with coordinated test updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:02:48Z

## DEC-034
- **Decision:** How to implement Step 9.2 runtime execution while minimizing scope.
- **Chosen Option:** Add a small stack-based IR evaluator in a dedicated `runtime` module, wire `tonic run <path>` through scan/parse/resolve/type/lower/evaluate, and execute a fixed `Demo.run` entrypoint with integer output rendering.
- **Confidence (0-100):** 77
- **Alternatives Considered:**
  - Evaluate AST directly and bypass IR.
  - Keep `run` as a placeholder and only satisfy the test with hardcoded output.
  - Add full `question`/`case` runtime semantics in this slice.
- **Reasoning:** Step 9.2 asks for a minimal evaluator loop and `tonic run` wiring. Evaluating existing IR keeps the architecture aligned with Step 8, satisfies the arithmetic smoke contract, and avoids overbuilding before Step 9.3 Result-propagation tests are locked.
- **Reversibility:** High — evaluator op coverage can be extended incrementally, and entrypoint selection can move to CLI flags/manifest later without rewriting the pipeline.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:07:10Z
