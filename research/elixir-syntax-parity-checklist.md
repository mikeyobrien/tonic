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
- [ ] float literals
- [ ] booleans (`true`/`false`)
- [ ] `nil`
- [ ] string literals as expressions (lexer token exists; expression support missing)
- [ ] string interpolation (`"#{expr}"`)
- [ ] heredocs (`"""..."""`)
- [ ] sigils (`~r`, `~s`, etc.)
- [ ] bitstring/binary syntax (`<<>>`)

---

## 2) Operators & Precedence

- [x] `+`
- [ ] arithmetic set (`-`, `*`, `/`)
- [ ] comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`)
- [ ] boolean operators (`and`, `or`, `not`)
- [ ] short-circuit operators (`&&`, `||`, `!`)
- [ ] concatenation operators (`<>`, `++`, `--`)
- [ ] membership/range (`in`, `..`)
- [ ] unary operators (`-x`, `+x`, `not x`, `!x`)
- [ ] precedence/associativity parity with Elixir

---

## 3) Data Structure Syntax (Literal Forms)

- [x] tuple construction (`tuple(a, b)` builtin exists; `{a, b}` literal syntax supported)
- [x] map construction (`map(k, v)` builtin exists; `%{k: v}` literal syntax supported)
- [x] keyword construction (`keyword(k, v)` builtin exists; `[k: v]` literal syntax supported)
- [x] list literals (`[1, 2, 3]`)
- [ ] map updates (`%{m | k: v}`)
- [ ] access syntax parity (`map.key`, `map[:key]`) where applicable

---

## 4) Pattern Matching Semantics

- [x] wildcard pattern (`_`)
- [x] atom patterns
- [x] integer patterns
- [~] tuple patterns (currently narrow runtime support)
- [x] list patterns
- [~] map patterns (lowering/runtime support for current single-entry map model)
- [ ] pin operator (`^var`)
- [ ] guards in patterns (`when`)
- [ ] match operator (`=`) in general expression/binding contexts
- [ ] exhaustive diagnostics parity for complex patterns

---

## 5) Functions & Clauses

- [x] named functions with fixed arity
- [ ] multi-clause function definitions (same name/arity, pattern-dispatched)
- [ ] pattern matching in function heads
- [ ] function guards (`def f(x) when ...`)
- [ ] default arguments (`\\`)
- [ ] private functions (`defp`)
- [ ] anonymous functions (`fn -> ... end`)
- [ ] capture operator (`&`, `&1`)
- [ ] function invocation parity for anonymous functions (`fun.(x)`)

---

## 6) Core Control Flow Forms

- [~] `case` (basic available)
- [ ] `if`
- [ ] `unless`
- [ ] `cond`
- [ ] `with`
- [ ] comprehensions (`for`)
- [ ] `try/rescue/catch/after`
- [ ] `raise`/exception forms

---

## 7) Module System & Compile-Time Forms

- [x] module declarations
- [ ] `alias`
- [ ] `import`
- [ ] `require`
- [ ] `use`
- [ ] module attributes (`@foo`, `@doc`, `@moduledoc`, `@spec`, etc.)
- [ ] protocol/impl syntax compatibility (`defprotocol`, `defimpl`) — optional for “syntax parity”, high for ecosystem parity

---

## 8) Tooling Parity (Language UX)

- [x] `run`, `check` command entrypoints
- [~] `fmt`/`test` command contracts (skeleton behavior exists)
- [ ] real formatter parity (`mix format`-like baseline)
- [ ] richer diagnostics (spans, hints, recovery quality)
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
