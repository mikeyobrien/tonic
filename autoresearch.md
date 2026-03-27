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
- **Current Best**: 46 focused testing UX checks green (run 40)
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
- **Run 31 (KEEP, metric=19, segment 2)**: Added `assert_contains` (string substring + list membership) and `assert_in_delta` (numeric proximity) to the Assert stdlib module, with structured failure rendering and 4 focused integration tests. Hypothesis: confirmed — expanding the assertion vocabulary with contains and delta checks covers the most common test patterns beyond equality, making Tonic tests more expressive without adding complexity.
- **Run 32 (KEEP, metric=22, segment 2)**: Added `--fail-fast` flag to `tonic test` that stops execution after the first test failure, using labeled loop breaks across file and test iterations, with 3 focused integration tests for early stop, all-pass continuation, and JSON output. Hypothesis: confirmed — fail-fast is a high-leverage workflow improvement that saves time in large suites by stopping at the first failure instead of running all remaining tests.
- **Run 33 (KEEP, metric=25, segment 2)**: Added ANSI colored output to `tonic test` text output — green for passing, red for failing, bold+red for failure headers — respecting `NO_COLOR` env var per no-color.org convention, with 3 focused integration tests for color presence, NO_COLOR stripping, and JSON ANSI-free verification. Hypothesis: confirmed — colored output makes test results instantly scannable with outsized UX impact relative to implementation complexity.
- **Run 34 (KEEP, metric=28, segment 2)**: Added `--seed <number>` flag to `tonic test` for reproducible randomized test ordering using splitmix64 PRNG and Fisher-Yates shuffle, with 3 focused integration tests for randomized order, deterministic reproduction, and JSON seed field. Hypothesis: confirmed — randomized test ordering detects implicit test-order dependencies that cause CI flakiness, and reproducible seeds make debugging easy.
- **Run 35 (KEEP, metric=31, segment 2)**: Added `setup/0` function support for test modules — an optional public `setup/0` function runs before each `test_*` function in its module, with setup failures marking tests as failed with "setup failed:" prefix, plus 3 focused integration tests for setup-before-each, setup-failure, and no-setup regression. Hypothesis: confirmed — shared test fixtures via `setup/0` enable DRY test authoring and follow established patterns from ExUnit/JUnit/pytest without adding complexity.
- **Run 36 (KEEP, metric=34, segment 2)**: Added `Assert.skip/0-1` for pending/skipped tests — skip() marks a test as skipped (yellow status, not counted as failure, doesn't trigger --fail-fast), with optional reason string, plus 3 focused integration tests for skip basic, skip with reason, and skip+fail-fast interaction. Hypothesis: confirmed — skip support is essential for WIP tests, known-broken tests, and platform-specific tests, following established patterns from ExUnit/@tag :skip, pytest/@pytest.mark.skip, and JUnit/@Disabled.
- **Run 37 (KEEP, metric=37, segment 2)**: Added `Assert.assert_raises/1-2` for testing error conditions — pure Tonic implementation using try/rescue with multi-clause private helpers, optional expected error pattern matching via host function, plus 3 focused integration tests for raise-passes, pattern-match, and JSON output. Hypothesis: confirmed — assert_raises is essential for testing error handling paths, following established patterns from ExUnit/assert_raise, pytest/pytest.raises, and JUnit/assertThrows.
- **Run 38 (KEEP, metric=40, segment 2)**: Added `Assert.assert_match/2-3` for map subset matching — checks if actual map contains all expected key-value pairs, with structured failure rendering showing missing/mismatched keys, falls back to equality for non-maps, plus 3 focused integration tests for subset pass, mismatch fail, and non-map equality. Hypothesis: confirmed — partial map matching is the most common assertion pattern for testing functions that return maps/structs, following established patterns from ExUnit/pattern match, pytest/dict subset, and Jest/expect.objectContaining.
- **Run 39 (KEEP, metric=43, segment 2)**: Added `--timeout <ms>` flag for per-test execution timeouts — spawns test execution in a thread with `mpsc::recv_timeout`, timeout applies to both setup/0 and test functions, timed-out tests marked as failed with "timed out after Xms" error, plus 3 focused integration tests for timeout-fail, fast-test-passes, and JSON output. Hypothesis: confirmed — per-test timeouts are essential for CI robustness and catching hangs/infinite loops, following established patterns from ExUnit/@tag timeout, pytest/--timeout, and JUnit/@Test(timeout).
- **Run 40 (KEEP, metric=46, segment 2)**: Added `teardown/0` function support for test modules — runs after each test regardless of outcome (even failures), teardown failures mark passing tests as failed with "teardown failed:" prefix, respects --timeout, plus 3 focused integration tests for teardown-runs, teardown-after-failure, and teardown-failure-marks-failed. Hypothesis: confirmed — teardown is the natural complement to setup/0 for test cleanup, following established patterns from ExUnit/on_exit, pytest/yield fixtures, JUnit/@AfterEach, and Jest/afterEach.
- **Run 41 (DISCARD, metric=46, segment 2)**: Attempted `Assert.capture_io/1` for testing IO output — added `host_capture_io_start`/`host_capture_io_stop` host functions and stdlib wrapper, but no integration tests were added, leaving the primary metric unchanged at 46. Hypothesis: incomplete — the implementation was missing tests, so the metric did not improve.

## Segment 2 — Common Libraries (from tonic-loops analysis)

### Objective

Study the tonic-loops repo and extract common patterns into reusable stdlib modules that benefit any Tonic application.

### Analysis Summary

Studied all 6 tonic-loops source files (main.tn, topology.tn, config.tn, memory.tn, harness.tn, pi_adapter.tn). Identified these duplicated patterns ranked by impact:

1. **JSON encoding/decoding** (~200+ lines hand-rolled across 3 modules) — highest impact
2. **TOML parsing** (~200 lines hand-rolled across 2 modules) — medium impact
3. **Shell quoting** (~30 lines duplicated across 3 modules) — lower impact
4. **List/string utilities** (list_contains, line_sep, read_if_exists, strip_quotes) — lower impact

### Metrics

- **Primary**: Number of common library functions passing focused tests
- **Current Best**: 311 focused Json+Toml+Shell+DateTime+Base64+Crypto+Uuid+Yaml+Env+Url+Path+File+Math+Regex+Random+Logger+Csv+Store+Bitwise+Hex+Access+Integer tests green + Http wrapper (run 49)
- **Secondary**: `cargo test` pass rate (must not regress), example apps 100%

### Benchmark Commands

```bash
cargo test --quiet json 2>&1 | tail -5
```

### Files in Scope

- `src/manifest_stdlib.rs` — stdlib source registration
- `src/stdlib_sources.rs` — stdlib source constants (alternative location)
- `src/interop.rs` — host_call dispatch
- `src/interop/system.rs` — system interop module
- `src/interop_tests.rs` — interop tests

### Constraints

- `cargo test` must pass (excluding pre-existing failures)
- All example apps must pass
- No new external dependencies (serde_json is already available)
- Follow existing stdlib patterns (host_call backed, optional lazy-loaded)

### What's Been Tried

- **Run 26 (KEEP, metric=14)**: Added `Json.encode/1`, `Json.decode/1`, and `Json.encode_pretty/1` as host-backed stdlib functions using serde_json, with Tonic value round-trip support for nil, bool, int, float, string, atom, list, map, tuple, and keyword lists, plus 14 focused unit tests. Hypothesis: confirmed — a Rust-backed Json module eliminates ~200 lines of fragile hand-rolled JSON across tonic-loops modules and provides a reliable foundation for any Tonic app needing structured data interchange.
- **Run 27 (KEEP, metric=25)**: Added `Toml.encode/1` and `Toml.decode/1` as host-backed stdlib functions using the toml crate, with Tonic value round-trip support for tables, arrays, strings, integers, floats, booleans, and datetimes (as strings), plus 11 focused unit tests. Hypothesis: confirmed — a Rust-backed Toml module eliminates ~200 lines of fragile hand-rolled TOML parsing across tonic-loops config.tn and topology.tn and provides reliable structured config parsing for any Tonic app.
- **Run 28 (KEEP, metric=37)**: Added `Shell.quote/1` and `Shell.join/1` as host-backed stdlib functions with POSIX single-quote wrapping, plus 12 focused unit tests. Hypothesis: confirmed — a Rust-backed Shell module eliminates ~40 lines of duplicated shell quoting across 4+ tonic-loops modules and provides safe command construction for any Tonic app that shells out.
- **Run 29 (KEEP, metric=45)**: Added `DateTime.utc_now/0`, `DateTime.unix_now/0`, and `DateTime.unix_now_ms/0` as host-backed stdlib functions using the `time` crate, plus 8 focused unit tests. Hypothesis: confirmed — a Rust-backed DateTime module eliminates shell-out `date` calls in tonic-loops memory.tn and provides reliable time access for any Tonic app needing timestamps.
- **Run 30 (KEEP, metric=57)**: Added `Base64.encode/1`, `Base64.decode/1`, `Base64.url_encode/1`, and `Base64.url_decode/1` as host-backed stdlib functions using the `base64` crate, with standard and URL-safe variants, plus 13 focused unit tests. Hypothesis: confirmed — a Rust-backed Base64 module provides reliable encoding/decoding for any Tonic app needing token handling, binary data interchange, or API payload encoding.
- **Run 31 (KEEP, metric=70)**: Added `Crypto.sha256/1`, `Crypto.hmac_sha256/2`, and `Crypto.random_bytes/1` as host-backed stdlib functions using sha2, hmac, and rand crates, with known test vector validation, plus 13 focused unit tests. Hypothesis: confirmed — a Rust-backed Crypto module provides reliable hashing, HMAC signing, and random byte generation for any Tonic app needing API authentication, content verification, or token generation.
- **Run 32 (KEEP, metric=70)**: Added `Http.get/1-2`, `Http.post/2-3`, `Http.put/2-3`, `Http.patch/2-3`, `Http.delete/1-2`, and `Http.request/4-5` as a pure Tonic wrapper over the existing `sys_http_request` host call. No new focused tests (wraps already-tested infrastructure). Hypothesis: confirmed — an ergonomic Http module eliminates raw `host_call(:sys_http_request, ...)` boilerplate and provides a clean API surface for any Tonic app making HTTP requests.
- **Run 33 (KEEP, metric=77)**: Added `Uuid.v4/0` as a host-backed stdlib function using the `rand` crate to generate RFC 4122 UUID v4 strings, with 7 focused unit tests covering format, version/variant bits, uniqueness, and error handling. Hypothesis: confirmed — a Rust-backed Uuid module provides reliable identifier generation for any Tonic app needing session ids, request correlation, or entity keys without shelling out to `uuidgen`.
- **Run 34 (KEEP, metric=88)**: Added `Yaml.encode/1` and `Yaml.decode/1` as host-backed stdlib functions using the `serde_yaml` crate, with Tonic value round-trip support for mappings, sequences, scalars, null, and tagged values, plus 11 focused unit tests. Hypothesis: confirmed — a Rust-backed Yaml module provides reliable YAML serialization for any Tonic app working with Docker, CI, Kubernetes, or other YAML-based configuration formats.
- **Run 35 (KEEP, metric=102)**: Added `Env.get/1-2`, `Env.fetch!/1`, `Env.set/2`, `Env.delete/1`, `Env.all/0`, and `Env.has_key/1` as host-backed stdlib functions with 14 focused unit tests. Hypothesis: confirmed — a dedicated Env module provides ergonomic environment variable access beyond the single `System.env/1` getter, enabling get-with-default, fetch-or-raise, set, delete, enumerate, and key-existence patterns for any Tonic app needing runtime configuration.
- **Run 36 (KEEP, metric=119)**: Added `Url.encode/1`, `Url.decode/1`, `Url.encode_query/1`, and `Url.decode_query/1` as host-backed stdlib functions with pure Rust RFC 3986 percent-encoding, plus 17 focused unit tests. Hypothesis: confirmed — a Rust-backed Url module provides reliable URL encoding/decoding and query string construction for any Tonic app using the Http module for API interactions.
- **Run 37 (KEEP, metric=132)**: Extended existing Path module with `Path.rootname/1` and `Path.split/1` host-backed functions, bringing Path to 14 focused unit tests. Hypothesis: confirmed — rootname (strip extension preserving directory) and split (decompose into components) complete the Path module's coverage of common filesystem path manipulation patterns needed by any Tonic app working with files.
- **Run 38 (KEEP, metric=144)**: Added `File` stdlib module wrapping existing System file operations (read/write/append/exists?/ls/ls_r/is_dir?/mkdir_p/rm_rf) plus 3 new host-backed functions (cp/rename/stat) with 12 focused unit tests. Hypothesis: confirmed — a dedicated File module provides ergonomic, familiar-named file operations that any Tonic app needs, eliminating the need to know System's non-obvious method names.
- **Run 39 (KEEP, metric=164)**: Added `Math` stdlib module with `pi/0`, `e/0`, `pow/2`, `sqrt/1`, `abs/1`, `min/2`, `max/2`, `log/1`, `log2/1`, `log10/1`, `sin/1`, `cos/1`, `tan/1`, `ceil/1`, `floor/1`, `round/1` as host-backed stdlib functions with 20 focused unit tests. Hypothesis: confirmed — a Rust-backed Math module provides essential mathematical operations for any Tonic app needing computation beyond basic arithmetic, with smart int/float return types and proper validation.
- **Run 40 (KEEP, metric=181)**: Added `Regex.match?/2`, `Regex.run/2`, `Regex.scan/2`, `Regex.replace/3`, `Regex.replace_all/3`, and `Regex.split/2` as host-backed stdlib functions using the `regex` crate (already a transitive dep), with 17 focused unit tests covering matching, captures, scan, replace with backreferences, split, invalid pattern errors, and edge cases. Hypothesis: confirmed — a Rust-backed Regex module provides essential text pattern matching for any Tonic app needing validation, extraction, or transformation beyond exact string equality.
- **Run 41 (KEEP, metric=192)**: Added `Random.integer/2`, `Random.float/0`, and `Random.boolean/0` as host-backed stdlib functions using the `rand` crate (already a dependency), with 11 focused unit tests covering integer range bounds, negative ranges, float range, boolean output, and error handling. Hypothesis: confirmed — a dedicated Random module provides ergonomic random number generation for any Tonic app needing randomness without manual conversion from Crypto.random_bytes or Enum.random.
- **Run 42 (KEEP, metric=207)**: Added `Logger.debug/1`, `Logger.info/1`, `Logger.warn/1`, `Logger.error/1`, `Logger.set_level/1`, `Logger.get_level/0` as host-backed stdlib functions with global AtomicU8 log level filtering and stderr output, with 15 focused unit tests. Hypothesis: confirmed — structured logging with level filtering replaces ad-hoc `IO.puts("[verbose]...")` patterns seen in tonic-loops apps.
- **Run 43 (DISCARD, metric=207)**: Attempted pure Tonic `Keyword` module with get/fetch/put/put_new/delete/has_key?/keys/values/merge/take/drop/to_map/keyword? — 13 functions operating on `[{atom, value}]` lists. No new focused Rust tests since it's entirely pure Tonic. Hypothesis: refuted — while the module adds useful functionality, the primary metric (focused test count) didn't improve, and pure Tonic wrappers need focused integration tests to count toward the metric.
- **Run 44 (KEEP, metric=224)**: Added `Csv.decode/1`, `Csv.encode/1`, `Csv.decode_maps/1`, `Csv.encode_maps/2` as host-backed stdlib functions with RFC 4180-compliant pure Rust parser/encoder (no new crate), handling quoted fields, escaped quotes, multiline fields, CRLF/LF, and auto-quoting on encode, with 17 focused unit tests. Hypothesis: confirmed — a Rust-backed Csv module provides reliable CSV parsing and encoding for any Tonic app doing data processing, reporting, or ETL.
- **Run 45 (KEEP, metric=239)**: Added `Store.new/0`, `Store.put/3`, `Store.get/2-3`, `Store.delete/2`, `Store.has_key?/2`, `Store.keys/1`, `Store.values/1`, `Store.size/1`, `Store.to_list/1`, `Store.clear/1`, `Store.drop/1` as host-backed stdlib functions using a global Mutex<HashMap> store, with 15 focused unit tests. Hypothesis: confirmed — an in-memory key-value Store module provides essential mutable state for any Tonic app needing caches, counters, accumulators, or session state beyond environment variables.
- **Run 46 (KEEP, metric=252)**: Added `Bitwise.band/2`, `Bitwise.bor/2`, `Bitwise.bxor/2`, `Bitwise.bnot/1`, `Bitwise.bsl/2`, `Bitwise.bsr/2` as host-backed stdlib functions with i64 native ops and 0..63 shift range validation, plus 13 focused unit tests. Hypothesis: confirmed — a dedicated Bitwise module provides essential bit manipulation for any Tonic app working with flags, permissions, masks, or binary protocols.
- **Run 47 (KEEP, metric=264)**: Added `Hex.encode/1`, `Hex.decode/1`, `Hex.encode_upper/1` as host-backed stdlib functions with pure Rust byte-to-nibble conversion (no new crate), handling encode/decode round-trips, odd-length and invalid-char errors, plus 12 focused unit tests. Hypothesis: confirmed — a dedicated Hex module complements Crypto (which outputs hex internally) by providing explicit hex encoding/decoding for any Tonic app working with cryptographic hashes, binary protocols, or debug output.
- **Run 48 (KEEP, metric=283)**: Added `Access.get_in/2`, `Access.put_in/3`, `Access.fetch/2`, `Access.keys/1` as host-backed stdlib functions for nested data traversal and manipulation, supporting string/atom keys for maps and integer keys for list indices, with recursive immutable update in put_in, plus 19 focused unit tests. Hypothesis: confirmed — a dedicated Access module eliminates manual nested pattern matching for any Tonic app working with deeply nested JSON/config structures.
- **Run 49 (KEEP, metric=311)**: Extended existing Integer module with `to_string/2` (base 2-36 conversion), `digits/1`, `undigits/1`, `gcd/2`, `is_even/1`, `is_odd/1`, `pow/2` as host-backed stdlib functions with overflow detection and input validation, bringing Integer to 28 focused unit tests total. Hypothesis: confirmed — a comprehensive Integer module provides essential number-theoretic and formatting operations for any Tonic app needing base conversion, digit manipulation, or integer math.

## Segment 3 — CLI Creation Stdlib Module

### Objective

Design and implement an elegant CLI creation stdlib module for Tonic, inspired by the best CLI libraries across languages (Click/Typer, clap, cobra, Thor, Oclif, OptionParser). Make `--output-json` first-class and provided for free on every CLI app.

### Design Philosophy

Studied the best CLI libraries: Click/Typer (Python), clap (Rust), cobra (Go), Thor (Ruby), Commander/Oclif (Node), OptionParser (Elixir). The sweet spot for Tonic is a **declarative, data-driven** approach:

1. Define your CLI as a **spec** (name, flags, args, subcommands)
2. Call `CLI.parse(spec, argv)` — automatic parsing, validation, help, and JSON output
3. `--help` and `--output-json` are injected for free on every spec
4. Errors produce helpful messages with exit codes

### Target API

```elixir
spec = CLI.spec(
  name: "myapp",
  version: "1.0.0",
  description: "A cool CLI tool",
  flags: [
    verbose: [type: :boolean, short: "v", doc: "Enable verbose output"],
    count: [type: :integer, short: "n", default: 1, doc: "Repetitions"]
  ],
  args: [
    file: [doc: "Input file", required: true]
  ]
)

case CLI.parse(spec, System.argv()) do
  {:ok, result} ->
    CLI.output(result, %{message: "done", count: result.flags.count})
  {:help, text} ->
    IO.puts(text)
  {:error, message} ->
    CLI.exit_error(message)
end
```

### Metrics

- **Primary**: Focused CLI module acceptance checks green
- **Current Best**: 20 (run 50)
- **Secondary**: `cargo test` pass rate (must not regress), example apps 100%

### Benchmark Commands

```bash
cargo test --quiet cli_module 2>&1 | tail -5
```

### Files in Scope

- `src/stdlib_sources.rs` — stdlib source registration
- `src/manifest_stdlib.rs` — stdlib source constants (alternative location)
- `src/interop.rs` — host_call dispatch
- `src/interop/system.rs` — system interop module (for host call registration)
- `src/interop_tests.rs` — interop tests

### Constraints

- `cargo test` must pass (excluding pre-existing failures)
- All example apps must pass
- No new external dependencies
- Follow existing stdlib patterns (host_call backed, lazy-loaded)
- `--output-json` must be automatic and free on every CLI spec
- Help generation must be automatic and free

### What's Been Tried

- **Run 50 (KEEP, metric=20)**: Implemented CLI.spec/1, CLI.parse/2, CLI.help_text/1, CLI.output/2, CLI.exit_error/1 as host-backed stdlib functions with automatic --help/-h, --version, --output-json injection, flag types (:boolean/:string/:integer/:float), short aliases, positional args (required/optional/rest), --no-* boolean negation, missing-value and unknown-flag errors, plus 20 focused unit tests. Hypothesis: confirmed — a declarative, data-driven CLI module with free --output-json and auto-help establishes the baseline for Tonic CLI creation.
