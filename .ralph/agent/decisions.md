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

## DEC-035
- **Decision:** What deterministic runtime failure contract to lock for Step 9.3 Result propagation RED coverage.
- **Chosen Option:** Add an end-to-end `tonic run` integration test where `Demo.run` evaluates `fail()?` and `fail` returns `err(7)`, asserting exit code `1`, empty stdout, and stderr `error: runtime returned err(7)`.
- **Confidence (0-100):** 70
- **Alternatives Considered:**
  - Assert only non-zero exit status without pinning stderr.
  - Use a runtime unit test instead of CLI integration.
  - Model propagation through `case` branches in the first Result runtime test.
- **Reasoning:** Step 9.3 explicitly asks for runtime Result propagation behavior and Step 9.4 will wire CLI exit mapping. A narrow CLI contract gives direct backpressure for both propagation and user-visible failure formatting without pulling in additional unsupported control-flow semantics.
- **Reversibility:** High — error message text can be revised later with coordinated fixture updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:10:35Z

## DEC-036
- **Decision:** How to implement Step 9.4 Result propagation semantics in runtime without overbuilding the evaluator.
- **Chosen Option:** Extend `RuntimeValue` with `ResultOk`/`ResultErr`, implement `ok`/`err` builtin calls, execute `question` as runtime short-circuit (`ok(v)` unwraps, `err(e)` returns early), and map top-level `ResultErr` in `tonic run` to `error: runtime returned err(<value>)` with exit code 1.
- **Confidence (0-100):** 76
- **Alternatives Considered:**
  - Represent propagated `err` as a dedicated `RuntimeError` variant instead of a runtime value.
  - Keep runtime values integer-only and special-case `ok/err/?` behavior directly in CLI handling.
  - Defer propagation support until `case` execution is implemented.
- **Reasoning:** The RED contract is CLI-visible and specifically exercises `err` propagation through `?`. Modeling Result as runtime values keeps call/stack behavior local to the evaluator, requires minimal surface change, and avoids conflating expected control flow with runtime faults.
- **Reversibility:** High — value representation can later move to a richer tagged value model or explicit control-flow enum with localized evaluator updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:14:07Z

## DEC-037
- **Decision:** How to refactor Step 9.5 runtime call/value internals to reduce allocation churn without changing public CLI behavior.
- **Chosen Option:** Route IR `call` execution through the operand stack (borrow tail slice for function calls, split stack once for builtin calls), change builtin argument handling to owned values, and remove `RuntimeValue` cloning in `ok/err` construction.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Keep existing `pop_args` buffer allocation + clone behavior and defer optimization to later runtime milestones.
  - Introduce a broader call-frame object with explicit locals/slots in this slice.
  - Add allocator instrumentation/bench harness before making any refactor changes.
- **Reasoning:** This is the narrowest reversible refactor that directly targets allocation churn now: function calls no longer allocate transient arg vectors, builtin calls consume moved values, and runtime semantics remain unchanged under existing integration tests.
- **Reversibility:** High — call dispatch remains localized to `src/runtime.rs`, so the evaluator can later migrate to richer frame layouts without touching CLI contracts.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:19:05Z

## DEC-038
- **Decision:** What Step 10.1 RED contract to use for tuple/map/keyword behavior before literal syntax exists.
- **Chosen Option:** Add an end-to-end `tonic run` integration fixture that builds collections via call-form constructors (`tuple(map(1, 2), keyword(3, 4))`) and expects rendered stdout `{%{1 => 2}, [3: 4]}`.
- **Confidence (0-100):** 71
- **Alternatives Considered:**
  - Add runtime unit tests for builtin constructors only (no CLI pipeline pressure).
  - Wait for tuple/map/keyword literal expression syntax before writing Step 10 tests.
  - Split into three independent integration fixtures (tuple/map/keyword) instead of one composite contract.
- **Reasoning:** The parser currently supports call expressions with integer arguments, so constructor-form fixtures are the narrowest reversible path to lock a red contract that pressures resolver, typing, lowering, runtime, and rendering together.
- **Reversibility:** High — constructor names, arities, and rendered format can be revised later with coordinated snapshot updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:21:52Z

## DEC-039
- **Decision:** How to introduce Step 10.2 tuple/map/keyword constructor support across resolver, typing, lowering, and runtime without expanding language syntax.
- **Chosen Option:** Treat `tuple`, `map`, and `keyword` as builtin call targets with fixed arity 2, infer their call type as `dynamic`, lower them as IR builtin calls, and represent runtime values with dedicated variants rendered as `{a, b}`, `%{k => v}`, and `[k: v]`.
- **Confidence (0-100):** 78
- **Alternatives Considered:**
  - Add first-class tuple/map/keyword expression syntax and type variants now.
  - Route constructors through synthetic stdlib functions instead of builtins.
  - Model map/keyword as generic list pairs without dedicated runtime variants.
- **Reasoning:** The RED contract is constructor-call based and needs end-to-end `tonic run` behavior immediately. Builtin call handling is the narrowest reversible change that unblocks the pipeline while preserving room for richer syntax and static typing later.
- **Reversibility:** High — constructor recognition, typing precision, and runtime value shapes are localized and can be evolved with coordinated fixture updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:26:32Z

## DEC-040
- **Decision:** What Step 10.3 RED protocol-dispatch contract to lock before protocol declarations/dispatch tables exist.
- **Chosen Option:** Add an end-to-end `tonic run` integration test that calls a new builtin-like `protocol_dispatch` over two concrete runtime values (`tuple(...)` and `map(...)`) and expects deterministic output `{1, 2}`.
- **Confidence (0-100):** 69
- **Alternatives Considered:**
  - Introduce full `defprotocol`/`defimpl` syntax in the RED fixture immediately.
  - Add runtime unit tests only for protocol dispatch internals without CLI coverage.
  - Use non-deterministic/partial assertions (exit status only) instead of a fixed output contract.
- **Reasoning:** Current parser grammar only supports integer/call/case expressions, so a call-form contract is the narrowest path that still pressures resolver, typing, lowering, runtime, and CLI output end-to-end for protocol-style dispatch behavior.
- **Reversibility:** High — the contract can migrate to richer protocol syntax later while preserving the same dispatch expectations.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:29:35Z

## DEC-041
- **Decision:** How to implement Step 10.4 protocol dispatch tables while the type system has no concrete collection/protocol types.
- **Chosen Option:** Treat `protocol_dispatch/1` as a builtin call across resolver/typing/IR lowering, and in runtime route it through a deterministic table mapping runtime kind labels (`tuple`, `map`) to integer implementation IDs (`1`, `2`).
- **Confidence (0-100):** 75
- **Alternatives Considered:**
  - Introduce `defprotocol`/`defimpl` syntax plus declaration-time tables now.
  - Encode dispatch using nested `case` logic in runtime instead of an explicit table.
  - Resolve dispatch IDs in typing and bake them into IR constants.
- **Reasoning:** The RED contract only requires protocol-style dispatch behavior for two concrete runtime values with deterministic output. A builtin + runtime table is the narrowest reversible GREEN slice that keeps end-to-end plumbing aligned with existing builtin call flow.
- **Reversibility:** High — dispatch tables can later be sourced from real protocol declarations without changing call sites.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:33:33Z

## DEC-042
- **Decision:** Which Step 10.5 RED fixture shape should lock pipe + Enum-style chaining without expanding parser/runtime scope.
- **Chosen Option:** Add `tonic run` integration coverage for `tuple(1, 2) |> Enum.stage_one() |> Enum.stage_two()` using local `Enum` module unary functions that ignore their input and return deterministic ints, expecting stdout `2`.
- **Confidence (0-100):** 73
- **Alternatives Considered:**
  - Use parameter-referencing function bodies (`protocol_dispatch(value)`) in the fixture.
  - Depend on an external stdlib `Enum` module before Step 11 loader work.
  - Use a non-pipeline fixture and defer pipe contract to a later step.
- **Reasoning:** Current parser/runtime slices do not support variable expression references or stdlib loading. This fixture still pressures the intended behavior (pipe should inject lhs into rhs call arity) while avoiding unrelated parse/module-loader blockers.
- **Reversibility:** High — fixture function bodies can later evolve to real value-transforming Enum operations once variable semantics and stdlib loading land.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:37:00Z

## DEC-043
- **Decision:** How to implement Step 10.6 pipe execution semantics for Enum-style call chains without expanding the parser/runtime value model.
- **Chosen Option:** Desugar `left |> rhs_call(...)` during typing and IR lowering by threading the left expression as the first argument to the rhs call target, while keeping rhs support scoped to call expressions only.
- **Confidence (0-100):** 76
- **Alternatives Considered:**
  - Introduce a dedicated `PipeCall` AST form in the parser and thread it through resolver/typing/lowering.
  - Execute pipe directly in runtime with a new IR `pipe` opcode instead of lowering to `call`.
  - Delay pipe execution until variable references/stdlib modules are richer.
- **Reasoning:** Threading lhs into existing call inference/lowering is the narrowest reversible change that unblocks the failing integration contract (`Enum.stage_one/0` becomes arity-1 at call-site) without introducing new runtime opcodes or parser surface area.
- **Reversibility:** High — this is localized to typing/lowering call handling and can later be replaced by richer pipe desugaring or dedicated IR.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:42:34Z

## DEC-044
- **Decision:** What Step 11.1 RED contract should define initial tonic.toml validation behavior for project-root execution.
- **Chosen Option:** Add a failing `tonic run .` integration test that writes `tonic.toml` with `[project]` but no `project.entry`, and assert deterministic failure `error: invalid tonic.toml: missing required key project.entry`.
- **Confidence (0-100):** 72
- **Alternatives Considered:**
  - Start with a manifest parser unit test only (no CLI/run-path pressure).
  - Lock a success contract for `tonic run <dir>` immediately, including module loading.
  - Validate a different missing-field contract first (for example `project.name`).
- **Reasoning:** The objective starts Step 11 with manifest parse/validation. This contract is the narrowest additive entry point that forces project-root detection plus deterministic validation messaging without pulling in multi-module loader behavior yet.
- **Reversibility:** High — required keys and message wording are localized to manifest validation and can evolve with coordinated test updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:44:29Z

## DEC-045
- **Decision:** How to add Step 11.2 project-root `tonic run` behavior without entangling manifest parsing into CLI command routing.
- **Chosen Option:** Introduce a dedicated `manifest` module that loads `tonic.toml`, validates required `project.entry`, and supplies run-source contents when the run path is a directory; keep file-path runs unchanged by delegating all source loading through `load_run_source(...)`.
- **Confidence (0-100):** 77
- **Alternatives Considered:**
  - Inline manifest parsing directly inside `handle_run` in `src/main.rs`.
  - Keep run-path behavior file-only and add a separate `--project` flag for root execution.
  - Implement a broader multi-module manifest loader in this slice.
- **Reasoning:** The active RED contract only requires deterministic validation for `tonic run .` when `project.entry` is missing. A focused loader module keeps `main.rs` from ballooning, preserves existing run contracts, and creates a clean seam for upcoming Step 11 module-graph work.
- **Reversibility:** High — manifest schema and loader behavior remain localized to one module and can be extended without changing CLI command parsing.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:48:14Z

## DEC-046
- **Decision:** What Step 11.3 RED contract should force project-level multi-module loading after manifest entry resolution.
- **Chosen Option:** Add a failing `tonic run .` integration fixture with `tonic.toml` entry `src/main.tn`, where `Demo.run` calls `Math.helper()` implemented in sibling file `src/math.tn`, and assert successful execution with stdout `1\n`.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Add only `manifest::load_run_source` unit tests for multiple files.
  - Use `tonic check` instead of `tonic run` and assert resolver success only.
  - Lock a failure-message contract instead of success behavior.
- **Reasoning:** Step 11.3 explicitly targets a multi-module project run fixture. This contract pressures the entire run pipeline (project loader + resolver/runtime visibility) and cleanly exposes the current gap (`Math.helper` unresolved) without introducing new syntax.
- **Reversibility:** High — fixture topology and expected output can evolve as import semantics/stdlib loading mature.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:49:43Z

## DEC-047
- **Decision:** How to implement Step 11.4 project module graph loading for `tonic run <project-root>` without introducing import syntax or a resolver-level dependency walker.
- **Chosen Option:** Keep manifest-driven entry loading, then recursively discover additional `.tn` files under the entry file’s directory, sort them deterministically, and concatenate their sources (entry first) into the run pipeline input.
- **Confidence (0-100):** 76
- **Alternatives Considered:**
  - Add parser/resolver-driven dependency walking based on call targets before loading files.
  - Load every `.tn` file from the project root (broader than needed, higher accidental coupling risk).
  - Introduce explicit module import declarations now and require manifests to enumerate modules.
- **Reasoning:** The active RED contract only requires sibling module visibility for `tonic run .`. Entry-dir recursive discovery is the narrowest reversible change that unblocks multi-module execution while preserving deterministic ordering and keeping scope aligned with Step 11.
- **Reversibility:** High — module discovery is isolated to `src/manifest.rs` and can be replaced later by explicit dependency graph loading.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:54:37Z

## DEC-048
- **Decision:** What RED contract should lock Step 11.5 lazy stdlib loading behavior without overreaching into cache or CLI flag work.
- **Chosen Option:** Add an integration suite `tests/run_lazy_stdlib_loading_smoke.rs` with module-load tracing expectations under `TONIC_DEBUG_MODULE_LOADS=1`, covering both (a) unreferenced `Enum` should not appear in load trace and (b) `Enum.identity()` should succeed only when optional stdlib module is lazy-loaded.
- **Confidence (0-100):** 73
- **Alternatives Considered:**
  - Add only a unit test in `manifest.rs` that inspects file lists without executing `tonic run`.
  - Lock only the positive path (`Enum` referenced) and skip the unreferenced trace assertion.
  - Defer trace assertions and assert resolver success/failure only.
- **Reasoning:** Step 11.5 explicitly calls for lazy-loading behavior tests with module-load tracing. A dual-path integration contract is the narrowest way to force both halves of the requirement (load on first use, do not pre-load when unused) while staying inside existing `tonic run` flow.
- **Reversibility:** High — trace token strings and optional module names can evolve with minimal test updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T02:58:33Z

## DEC-049
- **Decision:** How to implement Step 11.6 lazy optional stdlib loading and debug module-load tracing without widening the CLI/source-loading API.
- **Chosen Option:** Keep `manifest::load_run_source(...) -> Result<String, String>` unchanged, analyze project source AST to detect `Enum.*` call references, append a built-in optional `Enum.identity/0` module only when referenced and not already defined by the project, and emit `module-load project:<Module>` / `module-load stdlib:Enum` trace lines when `TONIC_DEBUG_MODULE_LOADS` is set.
- **Confidence (0-100):** 79
- **Alternatives Considered:**
  - Change loader API to return structured module metadata and move all tracing into `main.rs`.
  - Treat `Enum.identity` as a resolver/runtime builtin instead of a lazily loaded module.
  - Always preload optional stdlib modules for project-root runs.
- **Reasoning:** This is the narrowest additive change that satisfies both RED contracts (no eager stdlib load when unreferenced + successful referenced load with trace output) while preserving existing call paths and avoiding broader API churn.
- **Reversibility:** High — optional-module detection and trace emission remain localized to `src/manifest.rs` and can migrate to richer module-loader metadata later.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:01:57Z

## DEC-050
- **Decision:** What initial Step 12.1 RED contract should define cache-key variance and cache lookup semantics before on-disk wiring exists.
- **Chosen Option:** Add a new `src/cache.rs` unit-test contract that uses synthetic dimensions (`entry_hash`, `dependency_hash`, `runtime_version`, `target`, `flags`) and asserts (a) identical inputs produce identical keys, (b) changing one dimension changes the key, and (c) cache lookup transitions from miss to hit after storing payload.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Start with an end-to-end `tonic run` warm-cache integration contract before cache interfaces exist.
  - Test only key stability/variance and defer cache store behavior.
  - Add cache tests directly in `main.rs` instead of a dedicated cache module seam.
- **Reasoning:** Step 12 begins with cache hit/miss unit tests using synthetic keys. A dedicated module-level contract is the narrowest path that creates deterministic backpressure without forcing premature CLI/runtime cache plumbing.
- **Reversibility:** High — key shape and cache storage backend can change behind the same test seam.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:05:05Z

## DEC-051
- **Decision:** How to implement Step 12.2 cache-key derivation and cache storage seams without prematurely wiring on-disk persistence.
- **Chosen Option:** Derive `CacheKey` deterministically from the five synthetic dimensions using a length-prefixed concatenation format, and introduce a `CacheStorage` lookup/store interface implemented by an in-memory `CompileCache` (`HashMap<CacheKey, String>`).
- **Confidence (0-100):** 78
- **Alternatives Considered:**
  - Concatenate raw values with plain delimiters only (simpler but more collision-prone with delimiter-containing parts).
  - Pull in a hashing crate and hash dimensions into a digest key.
  - Skip a storage interface and keep direct `CompileCache` methods only.
- **Reasoning:** Length-prefixing gives deterministic key variance without new dependencies, and a small trait keeps Step 12.2 focused while creating a clean seam for later on-disk backend wiring in Step 12.4.
- **Reversibility:** High — key encoding and storage backend can be changed behind `CacheKey::from_parts` and `CacheStorage` without affecting callers.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:07:56Z

## DEC-052
- **Decision:** What RED integration contract should define Step 12.3 warm-run cache behavior before run-pipeline cache wiring exists.
- **Chosen Option:** Add `tests/run_cache_hit_smoke.rs` asserting two consecutive `tonic run .` executions under `TONIC_DEBUG_CACHE=1` report `cache-status miss` on first run and `cache-status hit` on second run while preserving program output.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Assert cache-hit behavior indirectly through timing differences (flaky and environment-dependent).
  - Add only a unit test in `src/cache.rs` and defer CLI integration coverage.
  - Assert warm-run success only without pinning cache trace semantics.
- **Reasoning:** Step 12.3 requires an integration contract that the second run uses cache. A debug-trace assertion is deterministic, avoids perf-flake risk, and creates direct backpressure for Step 12.4 cache plumbing without over-scoping into benchmark gates.
- **Reversibility:** High — trace token strings can be revised later with coordinated test updates while preserving the same warm-run cache intent.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:12:43Z

## DEC-053
- **Decision:** How to wire Step 12.4 on-disk cache into `tonic run` while keeping runtime behavior stable on cache read/write issues.
- **Chosen Option:** Compute a deterministic run cache key from source-derived hashes + runtime target metadata, attempt to load serialized IR from `.tonic/cache/<key>.ir.json` before compilation, and treat cache I/O/deserialize failures as cache misses (compile + continue) while emitting `cache-status miss|hit` only when `TONIC_DEBUG_CACHE` is set.
- **Confidence (0-100):** 77
- **Alternatives Considered:**
  - Fail `tonic run` immediately on cache read/write/deserialize errors.
  - Implement in-memory-only warm cache in-process and defer on-disk persistence.
  - Cache source text or typed AST instead of lowered IR.
- **Reasoning:** The active GREEN contract requires deterministic miss/hit tracing across separate `tonic run` invocations. On-disk IR artifacts satisfy cross-process reuse directly, and miss-on-error behavior avoids introducing new user-facing failures from optional cache plumbing.
- **Reversibility:** High — cache location, key dimensions, and artifact format are localized to `src/cache.rs` and `handle_run`, so behavior can be tightened later (e.g., corruption diagnostics) without changing CLI contracts.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:19:43Z

## DEC-054
- **Decision:** What Step 12.5 RED corruption contract should enforce cache recovery beyond a one-off compile fallback.
- **Chosen Option:** Add an integration test that warms cache, corrupts the artifact path by replacing the cache file with a directory, asserts the next run falls back with `cache-status miss`, and then requires a subsequent run to report `cache-status hit`.
- **Confidence (0-100):** 75
- **Alternatives Considered:**
  - Corrupt cache JSON payload only (already tolerated by current load path, likely green now).
  - Assert fallback success only, without requiring cache self-healing on a later run.
  - Add unit-only corruption tests in `src/cache.rs` instead of an end-to-end run contract.
- **Reasoning:** Step 12 requires corruption recovery behavior, and current implementation already handles invalid JSON misses. Directory-path corruption exposes a real gap: fallback succeeds but cache never recovers to hits because write failures are ignored. This red contract gives precise pressure for the next GREEN slice.
- **Reversibility:** High — corruption fixture shape and trace strings are localized to one integration test and can evolve with cache policy updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:22:30Z

## DEC-055
- **Decision:** What Step 13.1 RED integration contract should lock `tonic check/test/fmt` command-path behavior without overreaching into formatter/test-runner internals.
- **Chosen Option:** Add a new integration suite that executes `tonic check/test/fmt .` against a manifest-backed project root, asserting project-root path acceptance and deterministic success outputs (`check: ok`, `test: ok`, `fmt: ok`).
- **Confidence (0-100):** 71
- **Alternatives Considered:**
  - Lock only missing-argument usage errors for `test`/`fmt` and defer success-path contracts.
  - Assert permissive substring output (for example, just “contains ok”) instead of exact stdout lines.
  - Keep `check` file-only and avoid project-root path coverage in this slice.
- **Reasoning:** Step 13.1 explicitly targets command integration contracts for path + output/exit behavior. A shared project-root fixture keeps the RED scope narrow but still pressures the real missing behavior: `check` directory-path support and replacement of `test`/`fmt` skeleton outputs.
- **Reversibility:** High — output strings and path policy are localized to this integration suite and can be revised in later slices with coordinated fixture updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:40:15Z

## DEC-056
- **Decision:** How to satisfy Step 13.2 GREEN output contracts without breaking existing file-path `tonic check` success assertions.
- **Chosen Option:** Route `tonic check` source loading through `manifest::load_run_source` for directory support, but emit `check: ok` only for project-root directory invocations (no dump flags); keep file-path success output unchanged while adding real path validation + deterministic `test: ok` / `fmt: ok` outputs.
- **Confidence (0-100):** 76
- **Alternatives Considered:**
  - Emit `check: ok` for every successful `tonic check` invocation and update older success-output tests.
  - Keep `test`/`fmt` as skeleton output and only fix `check` directory loading.
  - Fully implement formatter/test-runner internals in this slice.
- **Reasoning:** The locked RED contract is specifically project-root command-path behavior. This option is the narrowest additive GREEN change that unblocks the new integration suite while avoiding broad output contract churn across earlier resolver/type-check tests.
- **Reversibility:** High — check-output policy is localized to `handle_check` and can be unified later when broader Step 13 command semantics are finalized.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:44:12Z

## DEC-057
- **Decision:** What Step 13.3 RED contract should lock `tonic verify run` mode-tag filtering behavior across auto/mixed/manual execution modes.
- **Chosen Option:** Add a new integration suite asserting scenario ID filtering by mode on a shared feature fixture: `auto` returns only `@auto`, `mixed` returns `@auto` + `@agent-manual`, and `manual` returns all tagged scenarios.
- **Confidence (0-100):** 74
- **Alternatives Considered:**
  - Assert only `mode_tags` metadata values and ignore filtered scenario results.
  - Use unit tests in `acceptance.rs` only (no CLI verify contract pressure).
  - Lock a single-mode contract (`auto` only) and defer mixed/manual expectations.
- **Reasoning:** Step 13.3 explicitly requires BDD mode tests for all three modes. A single integration fixture keeps scope narrow while creating deterministic backpressure on the real gap (verify currently emits unfiltered scenarios regardless of mode).
- **Reversibility:** High — filtering policy and fixture scenario IDs are localized to the verify path/tests and can evolve with coordinated contract updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:47:00Z

## DEC-058
- **Decision:** What Step 13.5 RED benchmark-gate contract should require from `tonic verify run` when measured performance exceeds v0 thresholds.
- **Chosen Option:** Add a CLI integration test that writes `benchmark_metrics` into `acceptance/step-13.yaml` with intentionally failing values (cold 74ms, warm 15ms, RSS 42MB) and asserts verify exits non-zero with structured JSON (`status: fail`, `benchmark.status: threshold_exceeded`, explicit threshold + measured fields).
- **Confidence (0-100):** 73
- **Alternatives Considered:**
  - Assert only non-zero exit status and stderr diagnostics without JSON structure.
  - Use a unit test in `acceptance.rs` for benchmark parsing and defer verify runner behavior.
  - Add a passing benchmark test first and defer failing-threshold contract to a later slice.
- **Reasoning:** Step 13.5 specifically requires a threshold-exceeded failure gate. Locking both exit behavior and minimal report schema prevents a superficial failure implementation and sets precise pressure for Step 13.6 to wire benchmark parsing + enforcement through the verify pipeline.
- **Reversibility:** High — fixture keys and benchmark report fields are localized to the new integration test and verify JSON assembly path.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:51:05Z

## DEC-059
- **Decision:** What Step 13.7 RED contract should define required manual-evidence behavior for `tonic verify run` in mixed mode.
- **Chosen Option:** Add an integration test that declares `manual_evidence.mixed` JSON file requirements in `acceptance/step-13.yaml` and asserts `tonic verify run step-13 --mode mixed` fails with structured report fields when the required evidence file is missing.
- **Confidence (0-100):** 72
- **Alternatives Considered:**
  - Assert failure only via stderr text and skip JSON report structure.
  - Gate evidence for all modes with one flat `manual_evidence_files` list.
  - Delay evidence requirements until after full verify workflow wiring.
- **Reasoning:** The objective explicitly calls out required manual evidence and mixed-mode failure behavior. A mode-scoped contract keeps the slice narrow while forcing real verify-run enforcement, not just metadata parsing.
- **Reversibility:** High — acceptance key shape and report fields are localized to verify contracts and can be adjusted with coordinated fixture updates.
- **Timestamp (UTC ISO 8601):** 2026-02-21T03:59:17Z
