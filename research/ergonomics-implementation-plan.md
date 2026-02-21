# Ergonomics Implementation Plan

This plan prioritizes resolving the top usability blockers identified in the ergonomics evaluation, focusing on unblocking dynamic program execution and control flow.

## Milestone 1: Variable and Identifier References in Expressions
- **Priority**: CRITICAL
- **Goal**: Enable the parser, IR, and runtime to process and evaluate variables and function parameters without trailing parentheses.
- **Scope**:
  - **In-scope**: `src/parser.rs` (`Expr`, `Parser::parse_atomic_expression`), `src/ir.rs` (`IrOp`, `IrFunction`), `src/runtime.rs` (`evaluate_function`, `RuntimeValue`).
  - **Out-of-scope**: Complex closures, lexical scoping beyond function body/pattern matches, reassignment/mutability.
- **Implementation tasks**:
  1. Update `Expr` in `src/parser.rs` to include a variant for variable references (e.g., `Expr::Variable(String)`).
  2. Modify `Parser::parse_atomic_expression` to parse identifiers without `()` as variables.
  3. Update `IrOp` in `src/ir.rs` with a corresponding variable reference variant.
  4. Lower `Expr::Variable` into the new `IrOp` variant within the compiler/resolver.
  5. Update `evaluate_function` in `src/runtime.rs` to lookup and resolve variable references from the current environment into a `RuntimeValue`.
- **Acceptance criteria**:
  - Identifiers can be used in expressions (e.g., `a + b`).
  - The compiler successfully lowers variable identifiers to the IR.
  - The runtime evaluates variables to their assigned `RuntimeValue`.
- **Test strategy**:
  - **Automated**: `cargo test` for new parser cases parsing standalone identifiers and for runtime evaluation of the variable `IrOp`.
  - **Manual**: `cargo run -- run examples/ergonomics/budgeting.tn` executes successfully without offset errors.

## Milestone 2: Runtime Execution for `case`
- **Priority**: CRITICAL
- **Goal**: Implement runtime execution for `case` statements to unblock control flow and pattern-matching driven routing.
- **Scope**:
  - **In-scope**: `src/runtime.rs` (`IrOp::Case`, `evaluate_function`, `RuntimeValue`), `src/ir.rs` (`IrPattern`).
  - **Out-of-scope**: Exhaustiveness checking at runtime, matching on advanced types not yet supported in IR patterns.
- **Implementation tasks**:
  1. Implement evaluation logic for `IrOp::Case` in `src/runtime.rs` within `evaluate_function`.
  2. Implement a pattern matching routine to compare a `RuntimeValue` against `IrPattern` variants (Tuples, Lists, Maps, Atoms, Wildcards).
  3. Bind variables from successful pattern matches into the environment for the branch execution.
  4. Evaluate the corresponding branch block upon a successful match.
- **Acceptance criteria**:
  - `IrOp::Case` executes without throwing "unsupported ir op" panics.
  - The correct branch is selected based on the input `RuntimeValue` and its block executes correctly.
- **Test strategy**:
  - **Automated**: `cargo test` verifying runtime evaluator behavior for `IrOp::Case` across supported `IrPattern` types.
  - **Manual**: `cargo run -- check examples/ergonomics/pattern_matching.tn` and `cargo run -- run examples/ergonomics/pattern_matching.tn` run successfully without crashing.

## Milestone 3: Atom Literals in Expressions + Integer Literals in Patterns
- **Priority**: HIGH
- **Goal**: Allow atom literals to be used as standard expressions and integer literals to be used directly in pattern matching.
- **Scope**:
  - **In-scope**: `src/parser.rs` (`Expr`, `Pattern`, `Parser::parse_atomic_expression`, `Parser::parse_pattern`), `src/ir.rs` (`IrOp`, `IrPattern`), `src/runtime.rs` (`RuntimeValue`).
  - **Out-of-scope**: Other literals like floats or string interpolations.
- **Implementation tasks**:
  1. Extend `Expr` and `Parser::parse_atomic_expression` to support atom literals (e.g., `:ok`) outside of patterns.
  2. Update `IrOp` to support atom expressions and ensure they evaluate to a `RuntimeValue` representing an atom in `src/runtime.rs`.
  3. Extend `Pattern` and `Parser::parse_pattern` to support integer literals.
  4. Update `IrPattern` to include integer literals.
  5. Extend runtime pattern matching in `src/runtime.rs` to handle numeric equality between a `RuntimeValue` and an integer `IrPattern`.
- **Acceptance criteria**:
  - Atoms can be returned directly or passed to functions/tuples as expressions (e.g., `tuple(:ok, 200)`).
  - `case` statements can match on specific numeric values.
- **Test strategy**:
  - **Automated**: `cargo test` to verify AST and IR representations for atom expressions and integer patterns, plus runtime evaluation and matching tests.
  - **Manual**: Create a test script using integer patterns and atom expressions, then verify with `cargo run -- run <script.tn>`.
