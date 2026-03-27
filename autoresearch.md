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
- **Current Best**: 28 focused REPL tests green (run 25, segment 1)
- **Secondary**: `autoresearch.checks.sh` pass, judge pass

### Benchmark Commands

```bash
cargo test --quiet --bin tonic repl::tests:: && cargo test --quiet --test repl_server
```

### What's Been Tried

- **Run 20 (KEEP, metric=14, judge=8/10)**: Extracted shared `ReplSession` state and added `tonic repl --listen <addr>` with newline-delimited JSON `eval` / `clear` / `load-file` requests, per-connection session isolation, and focused REPL server coverage. Hypothesis: confirmed — a reusable session core plus a minimal remote transport is a solid first substrate for nREPL-style development even before richer protocol features land.
- **Run 21 (KEEP, metric=16, judge=8/10)**: Added server-wide logical REPL session ids with session-addressed `eval` / `clear` / `load-file` plus `clone` / `close` lifecycle ops, and expanded focused unit + integration coverage for reconnect, clone, and close behavior. Hypothesis: confirmed — logical sessions that survive TCP reconnects materially improve the remote REPL substrate toward real nREPL-style workflows without blowing up the transport or evaluator core.
- **Run 22 (KEEP, metric=18, judge=8/10)**: Added a `describe` op that reports supported remote REPL ops plus logical-session semantics, and expanded focused unit + TCP integration coverage for advertised capabilities. Hypothesis: confirmed — capability discovery is a small but high-leverage step toward editor-friendly nREPL workflows because clients can now introspect the server before driving sessions.
- **Run 23 (KEEP, metric=20, judge=8/10)**: Routed host-side stdout/stderr through a scoped interop capture sink and surfaced captured output in remote `eval` / `load-file` responses, with focused unit and TCP integration coverage. Hypothesis: confirmed — returning request-scoped output makes the remote REPL materially closer to editor-driven nREPL workflows because clients can now observe emitted text without scraping server logs.
- **Run 24 (KEEP, metric=24, judge=8/10)**: Added request-scoped stdin plumbing for remote `eval` / `load-file`, threading optional JSON `stdin` through scoped interop input capture and focused unit + TCP integration coverage for connection-local and logical sessions. Hypothesis: confirmed — request-local stdin closes a major interactivity gap for editor-driven remote REPL workflows without widening scope beyond the existing session/capture substrate.
- **Run 25 (KEEP, metric=28, judge=8/10)**: Added optional request ids plus streamed stdout/stderr frames for remote `eval` / `load-file`, echoing ids in terminal responses and covering connection-local and logical-session streaming. Hypothesis: confirmed — request-addressable stream frames make the remote REPL materially closer to nREPL-style editor workflows by letting clients correlate asynchronous output with a specific in-flight evaluation without widening scope beyond the existing session/capture substrate.

## Segment 2 — Unit Testing UX

### Objective

Improve the Tonic unit testing UX so that writing, running, and debugging tests is ergonomic — with built-in assertions, structured failure output, test filtering, and timing.

### Metrics

- **Primary**: Focused unit testing UX acceptance checks green
- **Current Best**: 15 focused testing UX checks green (run 30)
- **Secondary**: `cargo test` pass rate (must not regress), example apps 100%

### Benchmark Commands

```bash
cargo test --quiet --bin tonic test_runner && cargo test --quiet --test test_runner_rich_diagnostics
```

### Files in Scope

- `src/test_runner.rs` — Test discovery, compilation, execution, reporting
- `src/cmd_test.rs` — CLI argument handling for `tonic test`
- `src/cmd_deps.rs` — Help text for `tonic test`
- `src/interop.rs` — Host call dispatch (for assertion builtins)
- `src/manifest_stdlib.rs` — Stdlib source registry
- `src/stdlib_sources.rs` — Stdlib module source constants
- `tests/test_runner_rich_diagnostics.rs` — Integration tests for test runner

### Constraints

- `cargo test` must pass (excluding pre-existing failures)
- All example apps must pass
- No new crate dependencies
- Assertions should use the existing `host_call` interop pattern
- Test failures must produce actionable output (expected vs actual)

### What's Been Tried

- **Run 26 (KEEP, metric=6, segment 2)**: Added a built-in Assert stdlib module with `assert/1`, `refute/1`, `assert_equal/2`, `assert_not_equal/2` host functions that produce structured `err({:assertion_failed, details})` failures with expected-vs-actual rendering, plus stdlib injection into the test runner and 6 focused integration tests. Hypothesis: confirmed — a built-in assertion library with structured failure output is the essential foundation for ergonomic test authoring in Tonic.
- **Run 27 (KEEP, metric=9, segment 2)**: Added `--filter <pattern>` to `tonic test` that substring-matches against test names, skipping non-matching tests before execution, with 3 focused integration tests for subset match, no match, and JSON+filter. Hypothesis: confirmed — test filtering is a high-leverage developer workflow improvement that lets authors run a single test during development without waiting for the full suite.
- **Run 28 (KEEP, metric=9, segment 2)**: Added per-test and total run timing to `tonic test`, displaying durations after each test status in text output (e.g. `test X ... ok (1.23ms)`) and `duration_ms` fields in JSON output, with timing validation integrated into existing JSON and text output test assertions. Hypothesis: confirmed — per-test timing completes the core testing UX feature set (assertions + filtering + timing) and enables performance regression detection without adding complexity.
- **Run 29 (KEEP, metric=12, segment 2)**: Added failure summary section to text output (grouped failures at end with numbered list and full errors) and `failures` array to JSON output, with 3 focused integration tests for mixed pass/fail summary, all-pass no-summary, and JSON failures array. Hypothesis: confirmed — grouping failures at the end of test output makes debugging large suites materially faster by eliminating the need to scroll through passing tests to find failure details.
- **Run 30 (KEEP, metric=15, segment 2)**: Added `--list` flag to `tonic test` that discovers and compiles tests but skips execution, outputting test names (text: one per line, JSON: `{"tests": [...]}`) with optional `--filter` combination, plus 3 focused integration tests. Hypothesis: confirmed — test discovery without execution is essential for editor/tooling integration and pairs naturally with `--filter` for CI matrix splitting.
