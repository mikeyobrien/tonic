# Autoresearch: LLM-First Compilation & Syntax Errors

## Objective

Make Tonic's compilation and syntax error messages LLM-first — optimized for LLM agents that need to self-correct code based on error output.

An LLM-first error answers: (1) What went wrong? (2) Where? (3) How to fix it?

## Metrics

- **Primary**: Number of error categories with actionable fix suggestions
- **Current Best**: 12/12 representative missing-end + unexpected-arrow + stray-block-keyword parser + CLI checks green (run 3)
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
