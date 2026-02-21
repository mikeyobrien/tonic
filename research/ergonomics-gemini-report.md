# Tonic Language Ergonomics Evaluation

## Program Inventory and Intent

1. **`budgeting.tn`**
   * **Intent:** A simple numeric workflow to calculate a total payroll amount using basic arithmetic and function arguments.
   * **Result:** Failed at compile time.

2. **`pipeline/` (Multi-module project)**
   * **Intent:** A data transformation pipeline spanning multiple modules (`Demo`, `Transform`, `Load`), passing data using the `|>` operator.
   * **Result:** Compiled and ran successfully.

3. **`error_propagation.tn`**
   * **Intent:** Demonstrate early return and error propagation using the `ok/err` functions and the `?` operator.
   * **Result:** Ran and successfully bubbled the error to the top-level runtime boundary.

4. **`pattern_matching.tn`**
   * **Intent:** Pattern-matching driven routing that decodes a tuple and branches using `case`.
   * **Result:** Compiled successfully, but failed at runtime due to an unsupported IR operation.

---

## Command Transcripts and Evidence

### 1. Budgeting (`examples/ergonomics/budgeting.tn`)
```bash
$ cargo run -- run examples/ergonomics/budgeting.tn
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/tonic run examples/ergonomics/budgeting.tn`
error: expected (, found PLUS at offset 65
```
*Note: The error occurs at `base + bonus` because identifiers without parentheses are not valid expressions.*

### 2. Pipeline (`examples/ergonomics/pipeline/`)
```bash
$ cd examples/ergonomics/pipeline && cargo run --manifest-path ../../../Cargo.toml -- run .
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running `/home/mobrienv/projects/tonic/target/debug/tonic run .`
1
```

### 3. Error Propagation (`examples/ergonomics/error_propagation.tn`)
```bash
$ cargo run -- run examples/ergonomics/error_propagation.tn
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/tonic run examples/ergonomics/error_propagation.tn`
error: runtime returned err(404)
```

### 4. Pattern Matching (`examples/ergonomics/pattern_matching.tn`)
```bash
$ cargo run -- check examples/ergonomics/pattern_matching.tn
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/tonic check examples/ergonomics/pattern_matching.tn`

$ cargo run -- run examples/ergonomics/pattern_matching.tn
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/tonic run examples/ergonomics/pattern_matching.tn`
error: unsupported ir op in runtime evaluator: case at offset 86
```

---

## Ergonomics Analysis

### What felt ergonomic
- **Piping (`|>`):** The pipe operator intuitively passes the left-hand side as the first argument to the right-hand side, identical to Elixir. This makes multi-stage data pipelines very clean and easy to read.
- **Error Propagation (`?`):** Appending `?` to a function call that returns `ok(val)` or `err(val)` works seamlessly. It naturally bubbles up errors to the caller without verbose `case` or `if/else` checks, making error-handling pipelines highly readable.
- **Project Structure:** Using a `tonic.toml` file to configure a multi-module project feels natural and handles imports across sibling files seamlessly.

### Where writing real programs became awkward/impossible
- **Cannot reference variables!** This is the single biggest blocker. The parser strictly expects `(` after any identifier in an expression (`parse_atomic_expression`). There is no AST or IR representation for evaluating a variable. This makes it impossible to use function parameters or pattern-matched bindings within the function body (e.g., `base + bonus` fails to parse because `base` is an identifier without `()`).
- **No integer or atom literals in expressions:** Attempting to return an atom directly (e.g., `tuple(:ok, 100)`) fails because atom literals are currently only supported within pattern matching, not as standalone expressions. 
- **No integer literals in pattern matching:** You cannot match on specific numbers (e.g., `{1, value} -> ...`). Patterns only support Tuples, Lists, Maps, Atoms, and bindings/wildcards.
- **`case` statements crash the runtime:** Even if you successfully write a `case` block that parses and lowers to IR, the runtime evaluator explicitly lacks support for `IrOp::Case` and immediately panics with an "unsupported ir op" error.

---

## Prioritized Language Improvements

1. **Implement Variable/Identifier Resolution (CRITICAL):**
   * Allow identifiers without parentheses to be parsed as variable references rather than throwing an error.
   * *Concrete Example:* `def add(a, b) do a + b end` must be syntactically valid and evaluate to the sum of the arguments.

2. **Implement `case` Execution in Runtime (CRITICAL):**
   * The IR lowering supports `case`, but the runtime throws an `unsupported ir op` error. Without this, control flow and routing are virtually impossible.
   * *Concrete Example:* `case value() do :ok -> 1 _ -> 0 end` should execute without a runtime panic.

3. **Support Literals in Expressions and Patterns (HIGH):**
   * Allow atoms like `:ok` to be used as standard expressions (e.g., `tuple(:ok, 200)`).
   * Allow integer literals to be used in pattern matching (e.g., `case status do 404 -> ... end`).

---

## Final Ergonomics Score: 2/10

**Justification:** While Tonic's high-level workflow designs—specifically the Elixir-like `|>` piping and Rust-like `?` error propagation—are highly elegant and syntactically pleasing, the language is fundamentally incapable of running real-world logic. The complete inability to reference variables or arguments inside expressions means that all programs must be entirely hardcoded with literals and nested function calls. Combined with a non-functional `case` evaluator at runtime, it is currently impossible to write a non-trivial, dynamic program. The foundation is excellent, but the "runtime semantics gap" must be closed for the language to be usable.