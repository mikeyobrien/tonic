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
