# Autoresearch: LLM-First Compilation & Syntax Errors

## Objective

Make Tonic's compilation and syntax error messages LLM-first — optimized for LLM agents that need to self-correct code based on error output.

An LLM-first error answers: (1) What went wrong? (2) Where? (3) How to fix it?

## Metrics

- **Primary**: Number of error categories with actionable fix suggestions
- **Current Best**: 166/166 representative parser + typing + resolver + CLI diagnostics checks green (run 19)
- **Secondary**: `cargo test` pass rate (must not regress), example apps 100%

## Benchmark Commands

### Cargo Tests
```bash
cargo test 2>&1 | tail -5
```
Expect: all pass except 3 pre-existing `cli_contract_compile` failures.

### Example Apps (must stay 100%)
```bash
cd ~/projects/tonic/examples/apps && for app in */; do
  app_name="${app%/}"
  if [ -f "$app_name/expected_output.txt" ]; then
    actual=$(cd /Users/rook/projects/tonic && cargo run --quiet --bin tonic -- run "examples/apps/$app_name" 2>/dev/null | sed 's/\x1b\[[0-9;]*m//g')
    expected=$(cat "$app_name/expected_output.txt")
    if [ "$actual" = "$expected" ]; then echo "PASS: $app_name"; else echo "FAIL: $app_name"; fi
  fi
done
```

## Files in Scope

- `src/cli_diag.rs` — CLI error formatting
- `src/resolver_diag.rs` — Resolution errors (E1001-E1015)
- `src/typing_diag.rs` — Type errors (E2001, E3001-E3002)
- `src/typing_infer.rs` — Type inference errors
- `src/parser/mod.rs` — Parse errors (no error codes currently)
- `src/parser/*.rs` — Parser modules
- `src/main.rs` — Compilation pipeline error assembly

## Constraints

- `cargo test` must pass (excluding 3 pre-existing `cli_contract_compile` failures)
- All 83 example apps must pass
- No new dependencies
- Don't change parser/type system logic — only error messages/diagnostics

## What's Been Tried

- **Run 1 (KEEP, metric=6/6)**: Added `[E0003] unexpected end of file: missing 'end'` diagnostics that anchor on the opening construct span for `defmodule`, `def`/`defp`, `if`/`unless`, `cond`, `with`, `for`, `case`, `try`, and anonymous `fn`. Added parser + CLI coverage for missing module/function/if `end` cases, with 6/6 representative missing-end checks green. Hypothesis: confirmed — dedicated EOF/missing-`end` diagnostics make truncated block failures much more actionable for LLM repair loops without changing parse semantics.
- **Run 2 (KEEP, metric=8/8)**: Added `[E0004] unexpected '->' outside a valid branch` diagnostics with a repair hint to wrap anonymous functions in `fn ... -> ... end` or move `->` into valid `case`/`cond`/`with`/`for`/`try` branches. Added parser + CLI coverage for bare `value -> value + 1`, bringing the representative parser + CLI diagnostic suite to 8/8 green. Hypothesis: confirmed — a dedicated unexpected-arrow diagnostic turns a generic parse failure into an actionable one-shot fix for LLM repair loops.
- **Run 3 (KEEP, metric=12/12)**: Added `[E0005]` diagnostics for stray `else`, `rescue`, `catch`, `after`, `end`, and `do` keywords in expression position, with repair hints that explain the missing opener or extra block keyword. Added parser + CLI coverage for representative stray `else` and `rescue` failures, bringing the representative parser + CLI diagnostic suite to 12/12 green. Hypothesis: confirmed — dedicated stray-block-keyword diagnostics convert generic parse failures into directly repairable feedback for LLM agents.
- **Run 4 (KEEP, metric=20/20)**: Added `[E0006] missing 'do'` diagnostics anchored on block-opening spans for `defmodule`, `def`/`defp`, `if`/`unless`, `cond`, `with`, `for`, `case`, and `try`, plus parser + CLI coverage for representative missing-`do` cases. Hypothesis: confirmed — construct-specific missing-`do` diagnostics give LLMs the exact opener, missing token, and repair location they need for one-shot block-header fixes.
- **Run 5 (KEEP, metric=27/27)**: Added `[E0007] missing '->'` clause diagnostics anchored to clause starts for `case`, `cond`, `with else`, `for reduce`, `try rescue`/`catch`, and anonymous `fn`, plus parser + CLI coverage for representative case/rescue/fn missing-arrow failures. Hypothesis: confirmed — construct-specific missing-arrow diagnostics tell LLMs exactly which clause form is incomplete and how to repair it in one shot.
- **Run 6 (DISCARD, metric=34/34)**: Tried actionable E2001 integer/operator mismatch diagnostics for arithmetic, comparison, unary-minus, and integer-only operators, plus representative typing + CLI coverage. Hypothesis: refuted — while the representative diagnostic suite improved to 34/34, the change regressed `typing::tests::infer_types_accepts_dynamic_operands_for_arithmetic`, so it changed typing behavior instead of purely improving diagnostics.
- **Run 7 (KEEP, metric=35/35)**: Added diagnostic-only E2001 bool-required and host-call atom-key mismatch hints, threaded them through existing mismatch sites, and expanded representative typing + CLI coverage for `not 1`, `case ... when 1`, function guards, and `host_call(1, 2)`. Hypothesis: confirmed — richer hints on already-failing bool/atom mismatch paths improve LLM repair guidance without changing typing semantics.
- **Run 8 (KEEP, metric=40/40)**: Added actionable E3001 `?`-requires-`Result` and E3002 non-exhaustive-`case` hints, threaded the new `?` hint selection through existing typing diagnostics, and expanded representative typing + CLI coverage for literal `1?`, mixed result/match flows, and missing wildcard `case` branches. Hypothesis: confirmed — richer result-propagation and exhaustiveness repair hints improve LLM guidance on existing failure paths without changing typing semantics.
- **Run 9 (KEEP, metric=49/49)**: Added diagnostic-only E2002 arity-mismatch helpers for exact/range/minimum arities, threaded call-expression offsets through existing typing inference error paths, and expanded representative typing + CLI coverage for module calls, builtins, guard builtins, and named captures. Hypothesis: confirmed — arity errors become materially more self-correctable for LLMs when they include accepted arities, repair guidance, and source locations without changing typing semantics.
- **Run 10 (KEEP, metric=79/79)**: Added diagnostic-only E1001 undefined-symbol suggestion plumbing for local/imported/module-qualified call typos, plus representative resolver + CLI coverage for typo, missing-import, and module-qualified miss cases. Hypothesis: confirmed — undefined-call failures become materially more one-shot-fixable for LLMs when E1001 points to the closest callable target or missing import/module guidance without changing resolution semantics.
- **Run 11 (KEEP, metric=84/84)**: Added shared parser-side `[E0008]` missing-map-entry `=>` diagnostics for map literals and map patterns, then expanded representative parser + CLI coverage for malformed `%{key value}` entries and broken map-pattern branches inside `case`. Hypothesis: confirmed — dedicated map-entry separator diagnostics give LLMs the exact missing token and repair pattern for a common `%{...}` syntax slip without changing parse semantics.
- **Run 12 (KEEP, metric=91/91)**: Added parser-side `[E0009]` capture and anonymous-function diagnostics for missing named-capture `/arity`, empty `&()` expressions, invalid `&0` placeholders, and mismatched `fn` clause arities, then expanded representative parser + CLI coverage for those failures. Hypothesis: confirmed — dedicated capture/fn diagnostics turn common `&` shorthand and multi-clause `fn` mistakes into one-shot-fixable parser feedback without changing parse semantics.
- **Run 13 (KEEP, metric=100/100)**: Added diagnostic-only E2001 numeric operand hints for bitwise operators, range bounds, and unary bitwise-not on already-failing concrete int-only mismatches, then expanded representative typing + CLI coverage for bool/string/nil numeric misuse. Hypothesis: confirmed — numeric operand failures become more one-shot-fixable for LLMs when E2001 explains the bad operand kind and suggests a concrete conversion or replacement without changing dynamic arithmetic semantics.
- **Run 14 (KEEP, metric=108/108)**: Added parser-side `[E0010]` missing-comma diagnostics for parenthesized/no-paren call arguments and function/protocol parameter lists, then expanded representative parser + CLI coverage for those separator mistakes. Hypothesis: confirmed — list-specific missing-comma diagnostics help LLMs repair common separator omissions in one shot instead of chasing generic parse or downstream arity errors.
- **Run 15 (KEEP, metric=114/114)**: Extended parser-side `[E0010]` missing-comma diagnostics to `with` clause lists plus `for` generator/option lists, then expanded representative parser + CLI coverage for those control-form separator mistakes. Hypothesis: confirmed — control-form missing-comma diagnostics help LLMs repair multi-clause `with`/`for` omissions in one shot instead of chasing misleading missing-`do` parser errors.
- **Run 16 (KEEP, metric=128/128)**: Extended parser-side `[E0002]` unclosed-delimiter diagnostics to grouped expressions, call/capture parentheses, index access, and function/protocol parameter lists, then expanded representative parser + CLI coverage for those missing-closer failures. Hypothesis: confirmed — construct-aware unclosed-delimiter diagnostics help LLMs repair missing `)`/`]` mistakes in one shot instead of chasing bare `expected )` / `expected ]` parser errors.
- **Run 17 (KEEP, metric=138/138)**: Added parser-side E0010/E0002 bitstring missing-comma and unclosed-delimiter diagnostics for bitstring literals and patterns, then expanded representative parser + CLI coverage for those failures. Hypothesis: confirmed — bitstring-specific separator and closing-delimiter diagnostics help LLMs repair `<<...>>` mistakes in one shot instead of chasing bare `expected >>` parser errors.
- **Run 18 (KEEP, metric=152/152)**: Extended parser-side E0010/E0002 diagnostics to alias child lists, import filter lists, and structured `raise(...)` keyword arguments, then expanded representative parser + CLI coverage for those failures. Hypothesis: confirmed — alias/import/raise list diagnostics help LLMs repair common separator and missing-closer mistakes in one shot instead of chasing bare delimiter or generic import-shape parser errors.
- **Run 19 (KEEP, metric=166/166)**: Extended parser-side E0010/E0002 diagnostics to remaining tuple/list/keyword/map/struct literals and patterns, then expanded representative parser + CLI coverage for those failures. Hypothesis: confirmed — construct-specific container separator and closing-delimiter diagnostics help LLMs repair common literal/pattern mistakes in one shot instead of chasing legacy generic parser errors.

## Segment 1 — nREPL bootstrap

### Objective

Bootstrap Clojure-style remote development by reusing Tonic's existing REPL evaluator behind a remotely drivable persistent session.

### Metrics

- **Primary**: Focused REPL server acceptance checks green
- **Current Best**: 16 focused REPL tests green (run 21, segment 1)
- **Secondary**: `autoresearch.checks.sh` pass, judge pass

### Benchmark Commands

```bash
cargo test --quiet --bin tonic repl::tests:: && cargo test --quiet --test repl_server
```

### What's Been Tried

- **Run 20 (KEEP, metric=14, judge=8/10)**: Extracted shared `ReplSession` state and added `tonic repl --listen <addr>` with newline-delimited JSON `eval` / `clear` / `load-file` requests, per-connection session isolation, and focused REPL server coverage. Hypothesis: confirmed — a reusable session core plus a minimal remote transport is a solid first substrate for nREPL-style development even before richer protocol features land.
- **Run 21 (KEEP, metric=16, judge=8/10)**: Added server-wide logical REPL session ids with session-addressed `eval` / `clear` / `load-file` plus `clone` / `close` lifecycle ops, and expanded focused unit + integration coverage for reconnect, clone, and close behavior. Hypothesis: confirmed — logical sessions that survive TCP reconnects materially improve the remote REPL substrate toward real nREPL-style workflows without blowing up the transport or evaluator core.
