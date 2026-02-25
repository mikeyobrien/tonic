# Tonic ↔ Elixir Syntax Parity Checklist (excluding BEAM/OTP)

Scope: language syntax, parser/AST shape, and syntax-facing CLI/tooling parity.

Out of scope: BEAM/OTP runtime model (processes, mailboxes, supervisors, GenServer, distribution, hot code upgrade, OTP app lifecycle).

Legend:
- [x] implemented and covered by tests/fixtures
- [~] partial / syntax-compatible but semantically limited or syntax-divergent
- [ ] missing

_Last updated: 2026-02-25_

---

## 1) Core language forms

- [x] `defmodule ... do ... end` (`tests/check_dump_ast_module.rs`)
- [x] `def name(args) do ... end` (`tests/check_dump_ast_module.rs`)
- [x] `defp` private functions (`tests/run_function_clauses_defaults_defp_smoke.rs`)
- [x] module-qualified calls (`Module.func(...)`) (`examples/parity/07-modules/module_qualified_calls.tn`)
- [x] pipe operator `|>` (`tests/check_dump_ast_pipe_chain.rs`)
- [x] `case ... do ... end` baseline (`tests/check_dump_ast_case_patterns.rs`)

## 2) Literals, expressions, operators

- [x] integers / floats (`examples/parity/01-literals/float_and_int.tn`)
- [x] atoms (`:ok`) (`examples/parity/01-literals/atom_expression.tn`)
- [x] booleans + `nil` (`examples/parity/01-literals/bool_nil_string.tn`)
- [x] strings (`"..."`) (`examples/parity/01-literals/bool_nil_string.tn`)
- [x] string interpolation (`"#{expr}"`) (`tests/check_dump_ast_string_interpolation.rs`)
- [x] heredocs (`"""..."""`) (`examples/parity/01-literals/heredoc_multiline.tn`)
- [~] sigils: `~s`/`~r` only (`examples/parity/99-stretch/sigils.tn`, `src/lexer.rs`)
- [~] bitstring literal parse support (`<<...>>`) but runtime lowered as list (`examples/parity/99-stretch/bitstring_binary.tn`)
- [ ] hex/octal/binary integer literals (`src/lexer.rs`)
- [ ] numeric separators (`1_000`) (`src/lexer.rs`)
- [ ] char literals (`?a`) (`src/lexer.rs`)

- [x] arithmetic `+ - * /` (`examples/parity/02-operators/arithmetic_basic.tn`)
- [x] comparison `== != < <= > >=` (`examples/parity/02-operators/comparison_set.tn`)
- [x] boolean keywords `and or not` (`examples/parity/02-operators/logical_keywords.tn`)
- [x] short-circuit `&& || !` (`examples/parity/02-operators/logical_short_circuit.tn`)
- [x] concatenation/list ops `<> ++ --` (`examples/parity/02-operators/concat_and_list_ops.tn`)
- [x] range and membership `..` / `in` (`examples/parity/02-operators/membership_and_range.tn`)
- [x] precedence baseline coverage (`tests/check_dump_ast_expressions.rs`)
- [ ] strict equality `===` / `!==` (`src/lexer.rs`)
- [ ] `div` / `rem` operator parity (`src/lexer.rs`)
- [ ] `not in` operator form (`src/parser.rs`)
- [ ] bitwise operator family (`src/lexer.rs`)
- [ ] stepped ranges (`..//`) (`src/lexer.rs`, `src/parser.rs`)

## 3) Collections, map/keyword forms, access

- [x] list literals (`[1,2,3]`) (`examples/parity/03-collections/list_literal.tn`)
- [x] tuple literals (`{a,b}`) (`examples/parity/03-collections/tuple_literal_and_match.tn`)
- [x] map literals with atom-label keys (`%{ok: 1}`) (`examples/parity/03-collections/map_literal_single_entry.tn`)
- [x] multi-entry maps (`examples/parity/99-stretch/multi_entry_map_literal.tn`)
- [x] keyword literals (`[ok: 1]`) (`examples/parity/03-collections/keyword_literal_single_entry.tn`)
- [x] multi-entry keywords (`examples/parity/99-stretch/multi_entry_keyword_literal.tn`)
- [x] map updates (`%{m | k: v}`) (`tests/check_dump_ast_map_update.rs`)
- [x] map access (`m.key`, `m[:key]`) (`examples/parity/03-collections/map_dot_and_index_access.tn`)
- [x] map fat-arrow entries (`%{"k" => v}`) (`src/lexer.rs`, `src/parser.rs`, `examples/parity/03-collections/map_fat_arrow_literal.tn`)
- [x] struct literals + updates (`%Foo{field: v}`, `%Foo{base | field: v}`) (`src/parser.rs`, `tests/check_dump_ast_struct_syntax.rs`, `examples/parity/03-collections/struct_literal_update_pattern.tn`)

## 4) Pattern matching

- [x] wildcard `_` (`tests/check_dump_ast_case_patterns.rs`)
- [x] literal patterns (atom/int/bool/nil/string) (`tests/check_dump_ast_case_patterns.rs`)
- [x] tuple patterns (`examples/parity/04-patterns/case_tuple_bind.tn`)
- [x] list patterns + cons/tail (`[h | t]`) (`examples/parity/99-stretch/list_cons_pattern.tn`)
- [x] map patterns with label syntax (`%{ok: v}`) (`examples/parity/99-stretch/map_colon_pattern.tn`)
- [x] map key/value patterns support Elixir syntax (`%{:ok => v}` / `%{"k" => v}`) (`src/parser.rs`, `examples/parity/04-patterns/case_map_arrow_pattern.tn`)
- [x] struct patterns (`%Foo{field: v}`) (`src/parser.rs`, `tests/check_dump_ast_struct_syntax.rs`, `examples/parity/03-collections/struct_literal_update_pattern.tn`)
- [x] pin operator `^var` (`examples/parity/04-patterns/pin_pattern_and_guard.tn`)
- [x] `when` guards in case/function branches (`examples/parity/04-patterns/pin_pattern_and_guard.tn`, `examples/parity/05-functions/function_guards_when.tn`)
- [x] match operator `=` (`examples/parity/04-patterns/match_operator_bindings.tn`)
- [x] non-exhaustive case diagnostics baseline (`tests/check_non_exhaustive_case.rs`)
- [ ] bitstring/binary patterns (`<<x::8, rest::binary>>`) (`examples/parity/99-stretch/bitstring_binary.tn`)

## 5) Functions and closures

- [x] named functions with fixed arity (`tests/check_dump_ast_module.rs`)
- [x] multi-clause function dispatch by head patterns (`examples/parity/05-functions/multi_clause_pattern_dispatch.tn`)
- [x] function guards (`when`) (`examples/parity/05-functions/function_guards_when.tn`)
- [x] default args (`\\`) (`examples/parity/05-functions/default_args.tn`)
- [x] private function visibility (`defp`) (`examples/parity/05-functions/private_defp_visibility.tn`)
- [x] anonymous functions (`fn ... -> ... end`) (`examples/parity/05-functions/anonymous_fn_capture_invoke.tn`)
- [x] capture shorthand (`&`, `&1`) (`examples/parity/05-functions/anonymous_fn_capture_invoke.tn`)
- [x] closure capture and invocation (`fun.(x)`) (`tests/run_anon_fn_capture_smoke.rs`)
- [x] guard builtin parity (`is_integer/1`, `is_float/1`, `is_number/1`, `is_atom/1`, `is_binary/1`, `is_list/1`, `is_tuple/1`, `is_map/1`, `is_nil/1`) with guard-only diagnostics (`src/guard_builtins.rs`, `src/resolver.rs`, `src/native_runtime/mod.rs`, `src/c_backend/terminator.rs`, `tests/run_guard_builtin_parity_smoke.rs`, `examples/parity/05-functions/guard_builtins_parity.tn`)
- [x] multi-clause anonymous functions (`fn ...; ... end`) (`src/parser.rs`, `tests/run_anon_fn_capture_smoke.rs`, `examples/parity/05-functions/function_capture_multi_clause_anon.tn`)
- [x] named function capture (`&Module.fun/arity`, plus local `&fun/arity`) (`src/parser.rs`, `tests/check_capture_diagnostics.rs`, `tests/run_anon_fn_capture_smoke.rs`, `examples/parity/05-functions/function_capture_named_arity.tn`)

## 6) Control flow

- [x] `if` / `if ... else` (`examples/parity/06-control-flow/if_unless.tn`)
- [x] `unless` / `unless ... else` (`examples/parity/06-control-flow/if_unless.tn`)
- [x] `cond` (`examples/parity/06-control-flow/cond_branches.tn`)
- [x] `with` and `with ... else` (`examples/parity/06-control-flow/with_happy_path.tn`, `with_else_fallback.tn`)
- [x] `for` single generator (`examples/parity/06-control-flow/for_single_generator.tn`)
- [x] `for` multi-generator (`examples/parity/06-control-flow/for_multi_generator.tn`)
- [x] `for ... into: list` (`examples/parity/06-control-flow/for_into.tn`)
- [x] `for ... into:` supports list/map/keyword destinations with deterministic tuple-shape constraints (`examples/parity/06-control-flow/for_into_map.tn`, `examples/parity/06-control-flow/for_into_keyword.tn`, `examples/parity/06-control-flow/for_into_runtime_fail.tn`)
- [x] `for reduce:` option (`examples/parity/06-control-flow/for_reduce.tn`, `tests/run_comprehensions_smoke.rs`)
- [x] `for` generator guards (`when`) (`examples/parity/06-control-flow/for_generator_guard.tn`, `tests/run_comprehensions_smoke.rs`)
- [x] `try/rescue/catch/after` baseline (`tests/check_dump_ast_try_raise.rs`, `tests/run_try_raise_smoke.rs`)
- [x] `raise` string forms (`raise("msg")`, `raise "msg"`) (`tests/check_dump_ast_try_raise.rs`)
- [x] exception struct/module raise forms (`raise FooError, message: ...`) with module rescue matching/value extraction (`src/parser.rs`, `src/runtime.rs`, `tests/check_dump_ast_try_raise.rs`, `tests/run_try_raise_smoke.rs`, `examples/parity/08-errors/structured_raise_rescue_module.tn`)

## 7) Module/compile-time forms

- [x] `alias Module, as: Name` (`examples/parity/07-modules/alias_import_use_require.tn`)
- [x] `import Module` + `import ... only:/except:` (`src/parser.rs`, `src/resolver.rs`, `tests/check_dump_ast_module_forms.rs`, `tests/run_import_only_except_semantics_smoke.rs`, `examples/parity/07-modules/import_only_except_semantics.tn`)
- [x] `require Module` scoped semantic validation (`src/resolver.rs`, `tests/run_use_require_semantics_smoke.rs`)
- [x] `use Module` scoped semantics (fallback import rewrite + target validation; macro expansion deferred) (`src/parser.rs`, `src/resolver.rs`, `tests/run_use_require_semantics_smoke.rs`, `examples/parity/07-modules/use_require_scoped_semantics.tn`)
- [~] module attributes (`@doc`, `@moduledoc`, custom attrs) parse/AST only (`tests/check_dump_ast_module_forms.rs`)
- [x] cross-file module resolution baseline (`tests/run_project_multimodule_smoke.rs`)
- [x] `import ... only:/except:` (`src/parser.rs`, `src/resolver.rs`, `tests/run_import_only_except_semantics_smoke.rs`)
- [x] `defprotocol` / `defimpl` syntax parity (`src/parser.rs`, `src/resolver.rs`, `src/ir.rs`, `tests/check_dump_ast_protocol_forms.rs`, `tests/run_protocol_defimpl_smoke.rs`)
- [ ] `__MODULE__` / `__ENV__` / `__CALLER__` (`src/` search)
- [ ] nested `defmodule` parity (`src/parser.rs`)
- [ ] `alias Foo` and `alias Foo.{Bar,Baz}` forms (`src/parser.rs`)

## 8) Syntax-facing tooling parity

- [x] `run` contract (usage errors vs runtime errors vs success) (`tests/cli_contract_run_command.rs`)
- [x] `check` contract + dump modes (`tests/cli_contract_common.rs`, `src/main.rs`)
- [x] `fmt` rewrite + idempotence (`tests/fmt_parity_smoke.rs`)
- [x] `fmt --check` non-mutating contract (`tests/fmt_parity_smoke.rs`)
- [x] `compile` contract and flag diagnostics (`tests/cli_contract_compile.rs`)
- [x] `test` command now executes discovered test files/functions with deterministic summary, non-zero failures, and optional `--format json` output (`src/test_runner.rs`, `src/main.rs`, `tests/test_runner_rich_diagnostics.rs`, `tests/check_test_fmt_command_paths.rs`)
- [x] parity fixture sweep (`examples/parity/catalog.toml`, `tests/run_parity_examples.rs`)
- [x] translated fixture smoke coverage (`tests/run_translated_fixtures_smoke.rs`)
- [x] stable diagnostic code families (`E1xxx`, `E2xxx`, `E3xxx`) (`src/resolver_diag.rs`, `src/typing_diag.rs`)
- [~] actionable hints exist but are not universal (`src/cli_diag.rs`)
- [x] line/column + snippet diagnostics parity for parser/resolver/typing failures in `check`/`test` (`src/cli_diag.rs`, `src/main.rs`, `src/resolver_diag.rs`, `src/typing_diag.rs`, `tests/test_runner_rich_diagnostics.rs`)
- [ ] docs generation command (`tonic docs`) / ExDoc-like output (`src/main.rs`)

---

## 9) Parity-complete exit checklist

- [ ] Idiomatic Elixir syntax examples (non-OTP) run without structural rewrites.
- [x] Map key/value syntax fully matches Elixir (`=>` forms in literals + patterns).
- [x] Remaining high-priority function/control-flow syntax gaps are closed (`&Module.fun/arity`, `for reduce`, generator guards, and non-list `into:`).
- [ ] Module compile-time forms have semantic parity beyond parse-only stubs (`use`, `require`, attributes).
- [x] Diagnostics provide line/column + contextual snippets for parser/resolver/typing errors in `check` and `test` paths.
- [ ] Docs generation parity exists (at least baseline extraction of `@doc` / `@moduledoc`).

## 10) Top 10 production-grade parity priorities

These are the highest-leverage gaps to close before calling Tonic "production-grade" for Elixir-style application development (still excluding BEAM/OTP runtime concerns).

1. [x] **Map `=>` key syntax parity (literals + patterns)**  
   `%{"k" => v}` literals and `%{"k" => x}` map-pattern forms are now supported alongside atom-label shorthand.

2. [x] **Struct syntax parity** (`%Module{...}`, updates, struct patterns)  
   `defstruct` forms, struct literal/update parsing, struct-pattern matching, resolver diagnostics, and runtime `__struct__` tagging are now wired end-to-end.

3. [x] **`defprotocol` / `defimpl` syntax + dispatch semantics**  
   Added first-class protocol declaration/implementation forms with resolver validation and runtime dispatch (tuple/map + struct-tagged values) while preserving `protocol_dispatch/1` builtin compatibility.

4. [x] **`use` and `require` semantic behavior (scoped parity)**  
   `require` now enforces compile-time module-target validation. `use` now applies deterministic scoped behavior (`use Module` acts as fallback import rewrite when no explicit imports) plus target validation. Full Elixir macro semantics (`__using__/1`, macro gating) remain deferred by design.

5. [x] **`import ... only:/except:` support**  
   `import Module, only: [...]` and `import Module, except: [...]` now parse, canonicalize, and resolve with deterministic malformed-payload/filtered/ambiguous diagnostics.

6. [x] **Guard builtin parity across backends**  
   Guard builtins (`is_integer/1`, `is_float/1`, `is_number/1`, `is_atom/1`, `is_binary/1`, `is_list/1`, `is_tuple/1`, `is_map/1`, `is_nil/1`) now share a central contract and deterministic guard-only diagnostics (`E1015`) across resolver, typing, interpreter runtime, and native backend lowering.

7. [x] **Function capture parity** (`&Module.fun/arity`, optional local `&fun/arity`) + richer anonymous-function clauses  
   Added parser/lowering support for named captures and multi-clause anonymous functions with guard-aware clause dispatch (`src/parser.rs`, `tests/check_capture_diagnostics.rs`, `tests/run_anon_fn_capture_smoke.rs`, `examples/parity/05-functions/function_capture_named_arity.tn`, `examples/parity/05-functions/function_capture_multi_clause_anon.tn`).

8. [x] **Comprehension parity completion** (`for reduce:`, generator guards, non-list collectables for `into:`)  
   Added end-to-end parser/lowering/runtime support for guarded generators, `reduce:` accumulator mode, and map/keyword `into:` collection semantics with deterministic failure contracts (`src/parser.rs`, `src/ir.rs`, `src/runtime.rs`, `src/c_backend/stubs.rs`, `tests/run_comprehensions_smoke.rs`, `tests/runtime_llvm_strings_lists_for.rs`, `examples/parity/06-control-flow/for_reduce.tn`, `examples/parity/06-control-flow/for_generator_guard.tn`, `examples/parity/06-control-flow/for_into_map.tn`, `examples/parity/06-control-flow/for_into_keyword.tn`).

9. [x] **Exception form parity** (`raise Module, opts`, structured rescue matching)  
   Added structured `raise Module, key: value` lowering to typed exception maps (`:__exception__`, `:message`, `:metadata`) plus rescue module matching (`Module ->`) and value extraction (`err in Module -> ...`) with deterministic invalid-form diagnostics (`src/parser.rs`, `src/runtime.rs`, `tests/check_dump_ast_try_raise.rs`, `tests/run_try_raise_smoke.rs`, `examples/parity/08-errors/structured_raise_rescue_module.tn`).

10. [x] **Developer-loop hardening: real `tonic test` runner + rich diagnostics**  
    `tonic test` now supports directory/file discovery (`*_test.tn` / `test_*.tn` and explicit file targets), deterministic pass/fail summaries, non-zero exits on failures, and `--format json` machine output. Parser/resolver/typing diagnostics now include line/column/snippet context while preserving stable diagnostic code families (`src/test_runner.rs`, `src/main.rs`, `src/cli_diag.rs`, `src/resolver_diag.rs`, `src/typing_diag.rs`, `tests/test_runner_rich_diagnostics.rs`).

## 11) Objective execution summary (elixir-prod-parity tasks 01-10)

Completed task files and corresponding commits on `main`:

| Task | Task file | Commit | Status |
|---|---|---|---|
| 01 | `.agents/tasks/tonic/elixir-prod-parity/01-map-fat-arrow-parity.code-task.md` | `17742f3` | ✅ complete |
| 02 | `.agents/tasks/tonic/elixir-prod-parity/02-struct-syntax-parity.code-task.md` | `fcd4a49` | ✅ complete |
| 03 | `.agents/tasks/tonic/elixir-prod-parity/03-protocol-defimpl-parity.code-task.md` | `4c8cb4d` | ✅ complete |
| 04 | `.agents/tasks/tonic/elixir-prod-parity/04-use-require-semantics.code-task.md` | `594a563` | ✅ complete |
| 05 | `.agents/tasks/tonic/elixir-prod-parity/05-import-only-except-parity.code-task.md` | `cad404c` | ✅ complete |
| 06 | `.agents/tasks/tonic/elixir-prod-parity/06-guard-builtins-backend-parity.code-task.md` | `b7346f0` | ✅ complete |
| 07 | `.agents/tasks/tonic/elixir-prod-parity/07-function-capture-anon-clauses.code-task.md` | `c0fe6a7` | ✅ complete |
| 08 | `.agents/tasks/tonic/elixir-prod-parity/08-comprehension-parity-completion.code-task.md` | `9015ff5` | ✅ complete |
| 09 | `.agents/tasks/tonic/elixir-prod-parity/09-exception-form-parity.code-task.md` | `bb57849` | ✅ complete |
| 10 | `.agents/tasks/tonic/elixir-prod-parity/10-test-runner-and-rich-diagnostics.code-task.md` | `5f66b94` | ✅ complete |

Remaining parity gaps (from unchecked/partial items above):

- Numeric literal parity gaps: hex/octal/binary forms, numeric separators, char literals.
- Operator parity gaps: strict equality (`===`/`!==`), `div`/`rem`, `not in`, bitwise family, stepped ranges.
- Advanced pattern/runtime gaps: bitstring/binary patterns.
- Compile-time/module gaps: full macro semantics for `use`, richer module attributes semantics, nested `defmodule`, additional `alias` forms, `__MODULE__/__ENV__/__CALLER__`.
- Tooling gap: `tonic docs` / ExDoc-like docs generation.
