# Tonic Examples Parity Plan (Comprehensive + Backward Runnability)

## Goal
Build a comprehensive example suite that tracks Elixir-syntax parity and guarantees each example is actually runnable in `tonic`.

### Definition of "runnable"
For each example:
- `tonic check <path>` returns expected exit code.
- `tonic run <path>` returns expected exit code.
- stdout/stderr are deterministic (exact match or documented substring contract).

### Current status (2026-02-23)
- ✅ Catalog + harness implemented (`examples/parity/catalog.toml`, `tests/run_parity_examples.rs`)
- ✅ All 48 parity examples are active
- ✅ Only one intentional non-zero run contract remains: `08-errors/question_operator_err_bubble.tn`
- ✅ `cargo test --test run_parity_examples` and `cargo test -q` pass

---

## 1) Comprehensive Example Catalog (target)

Directory plan:
- `examples/parity/01-literals/`
- `examples/parity/02-operators/`
- `examples/parity/03-collections/`
- `examples/parity/04-patterns/`
- `examples/parity/05-functions/`
- `examples/parity/06-control-flow/`
- `examples/parity/07-modules/`
- `examples/parity/08-errors/`
- `examples/parity/99-stretch/` (active stretch coverage)

### 01 — Literals (5)
1. `bool_nil_string.tn` — booleans/nil/string tuple rendering
2. `float_and_int.tn` — float literal behavior
3. `heredoc_multiline.tn` — triple-quote newline preservation
4. `interpolation_basic.tn` — `"hello #{1 + 2}"`
5. `atom_expression.tn` — `:ok` as expression value

### 02 — Operators & precedence (6)
6. `arithmetic_basic.tn` — `+ - * /`
7. `comparison_set.tn` — `== != < <= > >=`
8. `logical_keywords.tn` — `and or not`
9. `logical_short_circuit.tn` — `&& || !`
10. `concat_and_list_ops.tn` — `<> ++ --`
11. `membership_and_range.tn` — `in`, `..`

### 03 — Collections & access (6)
12. `tuple_literal_and_match.tn` — tuple literals + match
13. `list_literal.tn` — list constructor/literal run path
14. `map_literal_single_entry.tn` — `%{ok: 1}`
15. `keyword_literal_single_entry.tn` — `[ok: 1]`
16. `map_update_single_key.tn` — `%{m | k: v}`
17. `map_dot_and_index_access.tn` — `m.k` + `m[:k]`

### 04 — Patterns & case (6)
18. `case_atom_and_wildcard.tn` — `:ok` + `_`
19. `case_tuple_bind.tn` — `{:ok, v}`
20. `case_list_bind.tn` — `[head, tail]`
21. `case_map_arrow_pattern.tn` — `%{:ok -> v}`
22. `pin_pattern_and_guard.tn` — `[^x, y] when ...`
23. `match_operator_bindings.tn` — destructuring via `=`

### 05 — Functions, clauses, closures (5)
24. `multi_clause_pattern_dispatch.tn` — clause order and dispatch
25. `function_guards_when.tn` — guarded clauses
26. `default_args.tn` — `\\` defaults
27. `private_defp_visibility.tn` — local ok / cross-module rejected
28. `anonymous_fn_capture_invoke.tn` — `fn`, `&(&1...)`, `fun.(x)`

### 06 — Control flow (4)
29. `if_unless.tn` — if/unless lowering behavior
30. `cond_branches.tn` — cond branch selection
31. `with_happy_path.tn` — `with` chaining success
32. `with_else_fallback.tn` — `with ... else`

### 07 — Modules & project ergonomics (4)
33. `alias_import_use_require.tn` — module forms parse/resolve behavior
34. `module_attributes_doc.tn` — attributes parse/AST contract
35. `project_multifile_pipeline/` — tonic.toml entry + multi-file module chain
36. `module_qualified_calls.tn` — explicit `Mod.fun()` references

### 08 — Errors, result, interop (4)
37. `ok_err_constructors.tn` — `ok/err` values
38. `question_operator_success.tn` — `ok(v)?` path
39. `question_operator_err_bubble.tn` — deterministic runtime `err(...)`
40. `host_call_and_protocol_dispatch.tn` — builtin interop smoke

### 99 — Stretch parity (now active) (8)
41. `comments_hash.tn` — `# comment`
42. `noparen_calls.tn` — `fun 1` call style
43. `list_cons_pattern.tn` — `[h | t]`
44. `map_colon_pattern.tn` — `%{ok: v}` in pattern head
45. `multi_entry_map_literal.tn` — `%{a: 1, b: 2}`
46. `multi_entry_keyword_literal.tn` — `[a: 1, b: 2]`
47. `sigils.tn` — `~r/.../`
48. `bitstring_binary.tn` — `<<...>>`

---

## 2) Work Backwards: Runnability-First Execution Strategy

Start from the final requirement (all 40 core examples runnable), then move backward through dependencies.

## Phase A — Contracts first (before adding many files)
1. Add `examples/parity/catalog.toml` with:
   - `path`
   - `check_exit`
   - `run_exit`
   - `stdout` / `stderr_contains`
   - `status = active|blocked`
2. Add integration harness `tests/run_parity_examples.rs` that executes catalog entries.
3. Mark known blocked entries as `blocked` (not silently failing).

**Exit criteria:** one command validates the whole example corpus contract.

## Phase B — Fix shared blockers (highest leverage)
1. **Parser regression: list-pattern branches after prior case clauses**
   - Current symptom: `expected ], found COMMA`
   - Affects: `case_list_bind.tn`, existing `examples/ergonomics/pattern_matching.tn`
2. **Typing for map access/update arithmetic flow**
   - Current symptom: `expected int, found dynamic`
   - Affects: `map_dot_and_index_access.tn`, existing `run_map_update_access_smoke`
3. **Stale integration test cleanup**
   - `tests/check_dump_ast_map_update.rs` currently references non-existent `tonic_core` crate

**Exit criteria:** existing red tests become green:
- `check_dump_ast_case_patterns`
- `run_map_update_access_smoke`
- `check_dump_ast_map_update`

## Phase C — Add examples in dependency order
Order by least dependencies first:
1. 01 literals
2. 02 operators
3. 05 function basics (non-pattern-heavy)
4. 08 result/error
5. 03 collections
6. 04 patterns
7. 06 control flow
8. 07 modules/projects

For each new example:
- Add file
- Add catalog row
- Run harness
- If failing, add narrow parser/typing/IR/runtime regression test before fix

## Phase D — Promote stretch items into active set (completed)
Completed: after core #1-40 were green, #41-48 were implemented and promoted from blocked to active.

---

## 3) Backward Dependency Map (feature -> examples)

- **List-pattern parser robustness** -> #20, #31, existing ergonomics pattern example
- **Map access typing precision** -> #17, #16
- **Collection shape generalization (multi-entry maps/keywords)** -> #45, #46
- **Pattern syntax expansion (`[h|t]`, `%{k: v}` patterns)** -> #43, #44
- **Lexer feature expansion (comments/sigils/bitstrings)** -> #41, #47, #48

---

## 4) Milestones

### Milestone M1 (stabilize current parity)
- [x] Phase A + Phase B complete
- [x] Existing examples all runnable except intentionally error-propagation sample

### Milestone M2 (core comprehensive runnable)
- [x] All #1-40 implemented
- [x] Catalog harness fully green in CI

### Milestone M3 (stretch parity growth)
- [x] #41-48 promoted from blocked to active

---

## 5) Immediate next execution slice (completed)

1. [x] Create catalog harness (`catalog.toml` + `run_parity_examples.rs`).
2. [x] Fix parser list-pattern branch regression.
3. [x] Fix map access/update typing mismatch.
4. [x] Clean stale `tonic_core` test.
5. [x] Land first 10 parity examples (01 + 02 minus blocked items).
