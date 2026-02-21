# Memories

## Patterns

## Decisions

## Fixes

### mem-1771646688-a6f5
> failure: cmd=cargo test, exit=101, error='intermittent failure in acceptance::tests::load_acceptance_yaml_resolves_relative_feature_paths (missing acceptance/step-01.yaml) likely from cwd mutation race; immediate rerun passed', next='if it reappears, remove std::env::set_current_dir unit-test mutation or serialize those tests'
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771646538-6ae0
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/main.rs after Step 13.8 manual-evidence enforcement', next=run cargo fmt --all then re-run verification checks
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771646348-0f7c
> failure: cmd=rg --line-number 'evidence|manual' src tests acceptance -g '*.rs' -g '*.yaml' -g '*.feature', exit=2, error='rg failed because acceptance path does not exist in repo root', next=scope rg to existing directories or append || true for absence-tolerant discovery checks
<!-- tags: tooling, search, rg | created: 2026-02-21 -->

### mem-1771646345-190f
> failure: cmd=cargo test --test verify_manual_evidence_requirements, exit=101, error='verify run still passes in mixed mode when manual_evidence file is missing (status=pass, exit=0)', next=implement acceptance manual_evidence parsing + verify mixed/manual enforcement with fail status and structured manual_evidence report payload
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771646190-fdd1
> failure: cmd=cargo test, exit=101, error='acceptance unit test using set_current_dir for benchmark fixture intermittently read wrong cwd (missing acceptance/step-13.yaml) due global cwd races', next=avoid adding more cwd-mutating unit tests; keep benchmark contract in integration test fixture with current_dir on command process
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771646166-d916
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/main.rs after Step 13.6 benchmark threshold enforcement', next=run cargo fmt --all then re-run verification checks
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771645922-fdbe
> failure: cmd=cargo test --test verify_benchmark_gate_thresholds, exit=101, error='verify run reports pass when benchmark_metrics exceed thresholds (status=pass, exit=0)', next=implement verify benchmark threshold enforcement with fail status/non-zero exit and structured benchmark report payload
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771645803-be61
> failure: cmd=cargo test, exit=101, error='verify_feature_parser expected mixed mode to include @human-manual scenario after mode-filter implementation', next=update parser metadata coverage test to run with --mode manual while keeping mode-specific filtering in verify_mode_tag_filtering
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771645623-ed5b
> failure: cmd=cargo test --test verify_mode_tag_filtering, exit=101, error='verify run currently returns unfiltered scenarios for auto/mixed modes (includes @human-manual)', next=implement mode-based scenario filtering in verify runner before JSON report emission
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771645465-a076
> failure: cmd=cargo test --test check_test_fmt_command_paths, exit=101, error='repro showed check rejects project-root path and test/fmt still emit skeleton output', next=wire command handlers to load project-root sources and emit deterministic check/test/fmt ok contracts
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771645209-c1fb
> failure: cmd=cargo test --test check_test_fmt_command_paths, exit=101, error='check rejects project-root path and test/fmt still emit skeleton output contracts', next=implement check/test/fmt project-root path handlers and deterministic ok output contracts
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771644727-be45
> failure: cmd=find . -maxdepth 2 -type d | rg '\/spec($|/)', exit=1, error='rg returns 1 when no spec directory exists', next=append || true for absence-tolerant discovery checks
<!-- tags: tooling, search, rg | created: 2026-02-21 -->

### mem-1771644717-3914
> failure: cmd=env | rg -i 'spec_dir|SPEC_DIR', exit=1, error='no matches when optional env var absent', next=probe with rg || true when checking optional env vars
<!-- tags: tooling, search, rg | created: 2026-02-21 -->

### mem-1771644526-b6e4
> failure: cmd=cargo clippy --all-targets, exit=101, error='clippy::never_loop in src/main.rs parse_check_command_args (for-loop returns on first iteration)', next=replace loop with if let Some(argument)=args.iter().skip(1).next() and re-run clippy
<!-- tags: tooling, lint, rust | created: 2026-02-21 -->

### mem-1771644142-784a
> failure: cmd=cargo test --test run_cache_corruption_recovery_smoke, exit=101, error='cache artifact path corruption (directory at artifact file) yields repeated miss because run cache does not heal artifact path for subsequent hits', next=teach cache load/store path to repair directory-form artifact corruption so fallback compile run rewrites artifact and next run hits cache
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771643977-65d2
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/cache.rs after Step 12.4 cache wiring changes', next=run cargo fmt --all then re-run verification checks
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771643518-df69
> failure: cmd=cargo test --test run_cache_hit_smoke, exit=101, error='cache trace missing miss/hit markers on repeated run because run pipeline has no cache wiring', next=wire on-disk cache lookup/store into tonic run and emit TONIC_DEBUG_CACHE miss/hit traces
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771643099-2ae8
> failure: cmd=cargo test cache_key_changes_when_any_dimension_changes, exit=101, error='cache key variance assertion fails because changed target still yields identical empty key', next=implement CacheKey::from_parts derivation and in-memory cache lookup/store behavior for Step 12.2 GREEN
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771643087-6041
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/cache.rs after adding Step 12.1 RED cache contract tests', next=run cargo fmt --all before final verification in GREEN step
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771643083-90ae
> failure: cmd=cargo test cache_key_changes_when_any_dimension_changes, exit=101, error='CacheKey::from_parts currently returns same empty key for changed dimensions so synthetic key variance contract fails', next=implement deterministic cache key derivation from entry/dependency/runtime/target/flags and satisfy hit/miss cache tests
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771642893-58e9
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/manifest.rs after lazy stdlib loader implementation', next=run cargo fmt --all then re-run verification checks
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771642685-433a
> failure: cmd=cargo test --test run_lazy_stdlib_loading_smoke, exit=101, error='tonic run lacks module-load trace output and does not lazy-load Enum stdlib on demand (undefined symbol Enum.identity)', next=implement debug module-load tracing plus optional Enum stdlib lazy loading when call targets reference it
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771642197-0b84
> failure: cmd=cargo test --test run_project_multimodule_smoke, exit=101, error='project-root run only loads manifest entry file so sibling module Math.helper is unresolved in Demo.run', next=implement project module graph loading for tonic run . so resolver/runtime see entry plus sibling modules
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771642062-330c
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/manifest.rs after manifest loader implementation', next=run cargo fmt --all then re-run verification checks
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771641856-9048
> failure: cmd=cargo test --test run_manifest_validation, exit=101, error='tonic run treats project root path as source file and never validates tonic.toml project.entry', next=implement project-root run path with tonic.toml manifest parsing and deterministic missing project.entry diagnostic
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771641807-a2cd
> failure: cmd=ls -la examples, exit=2, error='examples directory absent at repo root', next=use temp fixture directories in integration tests instead of assuming repo examples/ path
<!-- tags: tooling, filesystem | created: 2026-02-21 -->

### mem-1771641699-3106
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/typing.rs and src/typing/tests.rs after pipe threading implementation', next=run cargo fmt --all then re-run verification checks
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771641419-aaa4
> failure: cmd=cargo test --test run_pipe_enum_smoke, exit=101, error='arity mismatch for Enum.stage_one: expected 1 args, found 0 in tonic run pipe chain fixture', next=implement pipe semantics across typing/lowering/runtime so rhs call receives lhs as first argument
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771641227-1581
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/ir.rs and src/runtime.rs after protocol dispatch implementation', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771640965-6a50
> failure: cmd=cargo test --test run_protocol_dispatch_smoke, exit=101, error='resolver rejects protocol_dispatch call with undefined symbol in Demo.run', next=extend builtin protocol dispatch handling across resolver/typing/ir/runtime and map tuple/map inputs to deterministic dispatch outputs
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771640759-a910
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/resolver.rs, src/runtime.rs, and src/typing/tests.rs after collection constructor support', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771640504-74b0
> failure: cmd=cargo test --test run_collections_smoke, exit=101, error='resolver rejects tuple/map/keyword constructor call with undefined symbol tuple', next=extend builtin symbol handling + typing/lowering/runtime support for tuple/map/keyword constructors and rendered values
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771640324-e751
> failure: cmd=cargo test --test run_result_propagation && cargo test && cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/runtime.rs after runtime call-path refactor', next=run cargo fmt --all and re-run verification checks
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771640285-3849
> failure: cmd=cargo test evaluate_builtin_ok_moves_nested_payload_without_cloning, exit=101, error='evaluate_builtin_call currently borrows args slice so builtin cannot move payload (type mismatch expected &[RuntimeValue], found Vec<RuntimeValue>)', next=refactor call path to pass owned builtin args and avoid cloning nested RuntimeValue payloads
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771640031-3c96
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/runtime.rs after Result runtime propagation changes', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771639822-c192
> failure: cmd=cargo test --test run_result_propagation, exit=101, error='runtime rejects err builtin call during tonic run instead of propagating err(reason) result', next=implement runtime ok/err/question semantics and map propagated err result to deterministic run failure output
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771639609-837d
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/runtime.rs after adding runtime evaluator', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771639357-e969
> failure: cmd=cargo test --test run_arithmetic_smoke, exit=101, error='tonic run still emits command skeleton instead of executing script', next=implement runtime evaluator and wire tonic run to execute entrypoint and print result
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771639186-44d8
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/ir.rs after adding op offsets', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771638988-3c74
> failure: cmd=cargo test --test check_dump_ir_source_map, exit=101, error='dump-ir output missing op offset fields in const_int/return snapshot', next=add source-offset metadata to IR op schema and lowering emission
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771638936-2cb7
> failure: cmd=ls -la examples, exit=2, error='examples directory absent at repo root', next=inspect fixtures under tests/ or create temp examples dirs in integration tests instead of assuming repository examples/
<!-- tags: tooling, filesystem | created: 2026-02-21 -->

### mem-1771638860-fef2
> failure: cmd=cargo test lower_ast_canonicalizes_call_target_kinds, exit=101, error='parser fixture used unsupported bare identifier expression in helper body', next=use value() call form in fixture before asserting call-target canonicalization
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771638385-0e5d
> failure: cmd=cargo test --test check_dump_ir_result_case, exit=101, error='dump-ir lowering rejects case with unsupported expression at offset 37', next=implement Expr::Question and Expr::Case lowering ops to satisfy snapshot contract
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771638263-b5e5
> failure: cmd=rg --line-number "dump-ast|dump-tokens|Usage:\n  tonic check" tests src/main.rs, exit=2, error='rg rejects literal newline escape in default mode', next=use plain pattern or -U multiline mode for newline matching
<!-- tags: tooling, search, rg | created: 2026-02-21 -->

### mem-1771637977-6064
> failure: cmd=cargo test --test check_dump_ir_smoke, exit=101, error='check rejects --dump-ir with usage error', next=add --dump-ir flag and IR dump pipeline
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771637775-4448
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/typing/tests.rs and src/typing_diag.rs', next=run cargo fmt --all then rerun --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771637663-8285
> failure: cmd=cargo test infer_types_harmonizes_result_and_match_diagnostics, exit=101, error='TypingError missing code()/message() accessors for Result/match diagnostic assertions', next=add centralized typing diagnostics surface with accessor methods and rerun targeted test
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771637473-53d8
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/typing/tests.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771636926-665e
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/typing.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771636258-1963
> failure: cmd=cargo test infer_types_accepts_explicit_dynamic_parameter_annotation parse_ast_rejects_dynamic_annotation_outside_parameter_positions, exit=1, error='unexpected argument parse_ast_rejects_dynamic_annotation_outside_parameter_positions', next=run each test in separate cargo test invocation
<!-- tags: tooling, testing, rust | created: 2026-02-21 -->

### mem-1771636164-e470
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/typing.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771635805-098a
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/parser.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771635321-15d3
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/typing.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771634774-8726
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/resolver_diag.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771634543-79db
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/lexer.rs, src/parser.rs, and src/resolver.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771633958-97e9
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in tests/check_undefined_symbol.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771633808-4177
> failure: cmd=ralph tools task close task-1771633643-f895 --format json, exit=2, error='unexpected argument --format', next=run ralph tools task close <id> without --format
<!-- tags: tooling, ralph, cli | created: 2026-02-21 -->

### mem-1771632903-85ae
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/parser.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771632588-79f8
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/parser.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771632288-be44
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/lexer.rs and src/main.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-21 -->

### mem-1771632050-0db0
> failure: cmd=ralph tools task close task-1771631998-5cef --format json, exit=2, error='unexpected argument --format', next=run ralph tools task close <id> without --format
<!-- tags: tooling, ralph, cli | created: 2026-02-21 -->

### mem-1771631915-68b9
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/lexer.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-20 -->

### mem-1771631811-6c8b
> failure: cmd=cargo test --lib lexer::tests::scan_tokens_assigns_spans_for_tokens_and_eof, exit=101, error='no library targets found in package tonic', next=run cargo test <test-name> without --lib for binary crate
<!-- tags: testing, rust, tooling | created: 2026-02-20 -->

### mem-1771631368-17f0
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/lexer.rs and src/main.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-20 -->

### mem-1771630828-9a03
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/acceptance.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-20 -->

### mem-1771630585-350e
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/main.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-20 -->

### mem-1771630211-61ab
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/main.rs and tests/verify_missing_acceptance.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-20 -->

### mem-1771629989-15a7
> failure: cmd=cargo fmt --all -- --check, exit=1, error='rustfmt diff in src/main.rs', next=run cargo fmt --all then re-run --check
<!-- tags: tooling, formatting, rust | created: 2026-02-20 -->

### mem-1771622442-c880
> failure: cmd=cargo --version, exit=127, error='cargo: command not found', next=use nix develop or rust toolchain bootstrap before running cargo tests
<!-- tags: tooling, error-handling, rust | created: 2026-02-20 -->

## Context
