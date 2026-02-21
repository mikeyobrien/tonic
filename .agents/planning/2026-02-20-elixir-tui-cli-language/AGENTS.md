# AGENTS.md — Tonic v0 LLM Coding Guide

Purpose: help agents with Elixir training write correct code for **Tonic v0** (Elixir-inspired, Rust runtime).

## TL;DR

- Write **Elixir-inspired syntax**, but do **not** assume full Elixir semantics.
- Tonic v0 is **not BEAM** and **not OTP-compatible**.
- Prefer **Result-first** flow: `ok(value)` / `err(reason)` and `?` propagation.
- Type system is **static with inference**, mostly strict, with explicit `dynamic` escape hatch.
- Tooling is `tonic ...`, not `mix ...`.

---

## Compatibility Delta (Elixir -> Tonic v0)

## Supported in v0 (language feel)

- `defmodule`, `def`, `if`, `case`, `cond`, `fn`
- Pattern matching in function heads and `case`
- Pipe operator `|>`
- Maps, tuples, keyword-list representation
- Protocols (v0 subset)
- Enum-style core collection operations (v0 subset)

## Not supported / deferred in v0

- `defmacro` and runtime macro expansion
- Runtime eval (`Code.eval_*`-style behavior)
- OTP process/supervision model (`GenServer`, supervisors, etc.)
- Full `mix` compatibility and Hex workflow parity
- BEAM interop assumptions

## Semantics that differ from typical Elixir assumptions

1. **Typing**
   - Static inference first.
   - Implicit coercions should be treated as invalid unless explicitly supported.
   - Use explicit `dynamic` only at well-defined boundaries.

2. **Error handling**
   - Primary path is Result-style (`ok`/`err` + `?`).
   - Exceptions/panics are for unrecoverable faults, not normal control flow.

3. **Runtime model**
   - Single-process CLI runtime model first.
   - No lightweight BEAM process-per-task assumptions.

---

## Tooling Expectations

Use these commands, not mix:

- `tonic run <file|module>`
- `tonic check <path>`
- `tonic test`
- `tonic fmt`
- `tonic cache clear`
- `tonic verify run <slice-id> --mode auto|mixed|manual`

Project manifest is `tonic.toml`.

---

## BDD + Acceptance (Required)

Acceptance is source-of-truth via Gherkin features under:
- `acceptance/features/<slice-id>.feature`

Scenario tags:
- `@auto` -> executable automated checks
- `@agent-manual` -> agent-run manual checks with structured evidence
- `@human-manual` -> optional human checks

A slice is not done unless:
1. automated gates pass,
2. required manual scenarios pass,
3. evidence is captured.

---

## Agent Coding Rules of Thumb

1. Prefer the simplest v0-compatible construct.
2. Avoid proposing deferred features unless explicitly asked.
3. If an Elixir idiom depends on macros/OTP/mix internals, propose a v0-compatible alternative.
4. Keep output deterministic and diagnostics actionable.
5. Before declaring completion, run `tonic verify ...` in the required mode.

---

## Quick Translation Hints

- Elixir `{:ok, v} | {:error, e}` style -> Tonic `ok(v) | err(e)` style.
- Elixir exception-based flow -> prefer Result return + `?` propagation.
- `mix` tasks -> `tonic` command set.
- Macro-heavy DSL patterns -> explicit functions/data structures in v0.

---

If uncertain, default to: **smaller surface area, explicit types, explicit errors, verifiable behavior**.
