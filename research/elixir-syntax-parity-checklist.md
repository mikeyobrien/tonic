# Elixir Syntax Parity Checklist (Minus BEAM/OTP)

Scope: language syntax + core semantics + CLI ergonomics.

Out of scope: BEAM/OTP runtime model (process mailbox semantics, supervisors, GenServer, distribution, hot code upgrade, OTP application lifecycle).

Legend:
- [x] implemented
- [~] partial / incompatible
- [ ] missing

---

## 0) Baseline Core

- [x] `defmodule ... do ... end`
- [x] `def name(args) do ... end`
- [x] module-qualified calls (`Module.func(...)`)
- [x] pipe operator (`|>`) with first-arg threading
- [x] postfix `?` error propagation operator (non-Elixir extension)
- [x] `case ... do ... end` (basic execution)

---

## 1) Literals & Basic Expressions

- [x] integer literals
- [x] atom literals (`:ok`) as expressions
- [x] float literals
- [x] booleans (`true`/`false`)
- [x] `nil`
- [x] string literals as expressions
- [x] string interpolation (`"#{expr}"`)
- [x] heredocs (`"""..."""`) baseline triple-quoted multiline strings
- [~] sigils (`~r`, `~s`, etc.) — baseline lexer/parser support for `~s`/`~r`; currently lowered as plain strings (no sigil-specific runtime semantics)
- [~] bitstring/binary syntax (`<<>>`) — baseline literal parsing/eval available; currently lowered to list values

---

## 2) Operators & Precedence

- [x] `+`
- [x] arithmetic set (`-`, `*`, `/`)
- [x] comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`)
- [x] boolean operators (`and`, `or`, `not`)
- [x] short-circuit operators (`&&`, `||`, `!`)
- [x] concatenation operators (`<>`, `++`, `--`)
- [x] membership/range (`in`, `..`)
- [x] unary operators (`-x`, `+x`, `not x`, `!x`)
- [~] precedence/associativity parity with Elixir (core operator table covered; exhaustive parity still pending)

---

## 3) Data Structure Syntax (Literal Forms)

- [x] tuple construction (`tuple(a, b)` builtin exists; `{a, b}` literal syntax supported)
- [x] map construction (`map(k, v)` builtin exists; `%{k: v}` literal syntax supported)
- [x] keyword construction (`keyword(k, v)` builtin exists; `[k: v]` literal syntax supported)
- [x] list literals (`[1, 2, 3]`)
- [x] map updates (`%{m | k: v}`)
- [x] access syntax parity (`map.key`, `map[:key]`) where applicable

---

## 4) Pattern Matching Semantics

- [x] wildcard pattern (`_`)
- [x] atom patterns
- [x] integer patterns
- [x] boolean/`nil`/string literal patterns
- [~] tuple patterns (currently narrow runtime support)
- [x] list patterns (including cons/tail form `[head | tail]`)
- [~] map patterns (`%{ok: v}` and `%{:ok => v}` forms supported, including multi-entry matching; exhaustive parity still pending)
- [x] pin operator (`^var`)
- [~] guards in patterns (`when`) — case branches + function clauses with boolean-expression guard subset
- [x] match operator (`=`) in general expression/binding contexts
- [ ] exhaustive diagnostics parity for complex patterns

---

## 5) Functions & Clauses

- [x] named functions with fixed arity
- [x] multi-clause function definitions (same name/arity, pattern-dispatched)
- [x] pattern matching in function heads
- [x] function guards (`def f(x) when ...`)
- [x] default arguments (`\\`)
- [x] private functions (`defp`)
- [x] anonymous functions (`fn -> ... end`)
- [x] capture operator (`&`, `&1`)
- [x] function invocation parity for anonymous functions (`fun.(x)`)

---

## 6) Core Control Flow Forms

- [~] `case` (basic available)
- [x] `if`
- [x] `unless`
- [x] `cond`
- [~] `with` (supports `<-` chaining with pattern mismatch fallback via optional `else` clauses)
- [~] comprehensions (`for`) — baseline single-generator list comprehensions (`for pattern <- list do expr end`) are supported; multi-generator and options (`into:` etc.) are pending
- [x] `try/rescue/catch/after` (baseline try/rescue/catch/after fully supported)
- [x] `raise`/exception forms (baseline raise)

---

## 7) Module System & Compile-Time Forms

- [x] module declarations
- [x] `alias`
- [x] `import`
- [x] `require`
- [x] `use`
- [~] module attributes (`@foo`, `@doc`, `@moduledoc`, `@spec`, etc.) — parser/AST storage for generic attrs + `@doc`/`@moduledoc`; no `@spec` semantics yet
- [ ] protocol/impl syntax compatibility (`defprotocol`, `defimpl`) — optional for “syntax parity”, high for ecosystem parity

---

## 8) Tooling Parity (Language UX)

- [x] `run`, `check` command entrypoints
- [~] `fmt`/`test` command contracts (`fmt` now formats source + `--check`; `test` command remains baseline contract)
- [x] real formatter parity baseline (`tonic fmt <path> [--check]` with deterministic rewrites + idempotence)
- [~] richer diagnostics (actionable hints for unsupported module-form options; broader span/recovery polish still pending)
- [x] translated fixture sweep parses/checks/runs (`tests/run_translated_fixtures_smoke.rs`)
- [ ] docs generation parity (`ExDoc`-like experience, minus OTP coupling)

---

## 9) High-Impact Implementation Order

1. **Expressions & operators**: strings/booleans/nil + core operator set + precedence.
2. **Literal syntax**: list/tuple/map/keyword literal forms.
3. **Pattern matching completion**: map/list runtime + pin + guards.
4. **Functions parity**: multi-clause, head patterns, default args, `defp`, anonymous fn/capture.
5. **Control flow parity**: `if/unless/cond/with`.
6. **Module compile-time forms**: `alias/import/require/use`, attributes.
7. **Tooling hardening**: formatter + diagnostics quality.

---

## 10) “Parity Complete” Exit Criteria

Mark this checklist complete when all are true:

- [ ] Representative examples from Elixir docs syntax chapters parse and execute (excluding BEAM/OTP-specific APIs).
- [ ] Function clauses + pattern matching + guards behave compatibly for covered constructs.
- [ ] Literal and operator syntax coverage is sufficient to translate idiomatic small Elixir snippets without structural rewrites.
- [ ] Test fixtures exist for every checked item above.
- [ ] CLI diagnostics are deterministic and actionable for unsupported/misused constructs.
