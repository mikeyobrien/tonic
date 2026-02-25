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
- [~] guard builtin parity incomplete (`is_integer/1`, etc. not fully wired in compiled backend) (`src/c_backend/terminator.rs`)
- [ ] multi-clause anonymous functions (`fn ...; ... end`) (`src/parser.rs`)
- [ ] named function capture (`&Module.fun/arity`) (`src/parser.rs`)

## 6) Control flow

- [x] `if` / `if ... else` (`examples/parity/06-control-flow/if_unless.tn`)
- [x] `unless` / `unless ... else` (`examples/parity/06-control-flow/if_unless.tn`)
- [x] `cond` (`examples/parity/06-control-flow/cond_branches.tn`)
- [x] `with` and `with ... else` (`examples/parity/06-control-flow/with_happy_path.tn`, `with_else_fallback.tn`)
- [x] `for` single generator (`examples/parity/06-control-flow/for_single_generator.tn`)
- [x] `for` multi-generator (`examples/parity/06-control-flow/for_multi_generator.tn`)
- [x] `for ... into: list` (`examples/parity/06-control-flow/for_into.tn`)
- [~] `for ... into:` only supports list destination (`examples/parity/06-control-flow/for_into_runtime_fail.tn`)
- [ ] `for reduce:` option (`examples/parity/06-control-flow/for_reduce_fail.tn`)
- [ ] `for` generator guards (`when`) (`src/parser.rs`)
- [x] `try/rescue/catch/after` baseline (`tests/check_dump_ast_try_raise.rs`, `tests/run_try_raise_smoke.rs`)
- [x] `raise` string forms (`raise("msg")`, `raise "msg"`) (`tests/check_dump_ast_try_raise.rs`)
- [ ] exception struct/module raise forms (`raise FooError, message: ...`) (`src/parser.rs`)

## 7) Module/compile-time forms

- [x] `alias Module, as: Name` (`examples/parity/07-modules/alias_import_use_require.tn`)
- [~] `import Module` baseline only (no `only:`/`except:`) (`src/parser.rs`, `tests/check_dump_ast_module_forms.rs`)
- [x] `require Module` scoped semantic validation (`src/resolver.rs`, `tests/run_use_require_semantics_smoke.rs`)
- [x] `use Module` scoped semantics (fallback import rewrite + target validation; macro expansion deferred) (`src/parser.rs`, `src/resolver.rs`, `tests/run_use_require_semantics_smoke.rs`, `examples/parity/07-modules/use_require_scoped_semantics.tn`)
- [~] module attributes (`@doc`, `@moduledoc`, custom attrs) parse/AST only (`tests/check_dump_ast_module_forms.rs`)
- [x] cross-file module resolution baseline (`tests/run_project_multimodule_smoke.rs`)
- [ ] `import ... only:/except:` (`src/parser.rs`)
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
- [~] `test` command is contract-stable but currently syntax-load stub (no rich test runner semantics) (`src/main.rs`, `tests/check_test_fmt_command_paths.rs`)
- [x] parity fixture sweep (`examples/parity/catalog.toml`, `tests/run_parity_examples.rs`)
- [x] translated fixture smoke coverage (`tests/run_translated_fixtures_smoke.rs`)
- [x] stable diagnostic code families (`E1xxx`, `E2xxx`, `E3xxx`) (`src/resolver_diag.rs`, `src/typing_diag.rs`)
- [~] actionable hints exist but are not universal (`src/cli_diag.rs`)
- [ ] line/column + snippet diagnostics parity (`src/typing_diag.rs`)
- [ ] docs generation command (`tonic docs`) / ExDoc-like output (`src/main.rs`)

---

## 9) Parity-complete exit checklist

- [ ] Idiomatic Elixir syntax examples (non-OTP) run without structural rewrites.
- [x] Map key/value syntax fully matches Elixir (`=>` forms in literals + patterns).
- [ ] Remaining function/control-flow syntax gaps are closed (`&Module.fun/arity`, `for reduce`, etc.).
- [ ] Module compile-time forms have semantic parity beyond parse-only stubs (`use`, `require`, attributes).
- [ ] Diagnostics provide line/column + contextual snippets for syntax/type errors.
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

5. [ ] **`import ... only:/except:` support**  
   Production codebases rely on constrained imports to avoid namespace collisions and keep readability.

6. [ ] **Guard builtin parity across backends**  
   Fully support guard builtins like `is_integer/1`, `is_binary/1`, `is_list/1`, etc. in both interpreter and native/compiled paths.

7. [ ] **Function capture parity** (`&Module.fun/arity`) + richer anonymous-function clauses  
   Widely used in pipelines and callback APIs; required for idiomatic functional composition.

8. [ ] **Comprehension parity completion** (`for reduce:`, generator guards, non-list collectables for `into:`)  
   Necessary for data-transformation heavy application code.

9. [ ] **Exception form parity** (`raise Module, opts`, structured rescue matching)  
   Required for consistent production error design and interoperability with richer exception types.

10. [ ] **Developer-loop hardening: real `tonic test` runner + rich diagnostics**  
    Implement actual test discovery/execution plus line/column/snippet diagnostics to support large-team maintenance and CI triage.
