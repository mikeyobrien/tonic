# Tonic Autoresearch Plan: Make Tonic the Best Production-Ready Language for LLMs

## Executive Summary

**Objective**: Make Tonic the best production-ready language for LLM models to code with by systematically improving documentation, error messages, stdlib completeness, and tooling to maximize LLM success rate.

**Current Status**: Tonic has strong foundations (Elixir-inspired syntax with 97.5% Pass@1 potential per AutoCodeBench), but gaps exist between:
- What PROMPT.md claims vs. what's actually implemented
- Interpreter vs. native backend parity
- Lazy stdlib loading vs. actual module exposure
- Documentation vs. runtime behavior

**Primary Metric**: LLM pass rate on Tonic app generation (currently 100.0% at 64/64 in run 26)

---

## Current State Analysis

### What's Already Working

1. **Core Language Features**
   - Elixir-inspired syntax (pattern matching, comprehensions, pipelines)
   - Module system with lazy stdlib loading
   - Runtime value types: lists, maps, tuples, keywords, atoms, closures
   - Both interpreter and native compile paths

2. **Existing Stdlib Modules** (in `src/manifest_stdlib.rs`)
   - `System` - host-backed system primitives
   - `String` - host-backed string operations
   - `Path` - host-backed path manipulation
   - `IO` - host-backed I/O operations (puts, gets, inspect, ansi colors)
   - `List` - **pure Tonic implementation** (first, last, flatten, zip, unzip, delete, duplicate, insert_at, wrap)
   - `Map` - **hybrid**: host-backed primitives (keys, values, merge, drop, take, get, put, delete, has_key) + pure Tonic helpers (to_list, new, from_list)
   - `Enum` - **mostly pure Tonic** (60+ functions: count, sum, join, sort, reverse, take, drop, chunk_every, unique, into, map, filter, reduce, find, any, all, min, max, flat_map, zip, with_index, each, at, member, reject, sort_by, group_by, min_by, max_by, frequencies, uniq_by, map_join, dedup, intersperse, zip_with, take_while, drop_while, chunk_by, scan, split, count_by)

3. **Infrastructure**
   - Lazy stdlib loading via `src/manifest.rs`
   - Host function registry in `src/interop.rs`
   - Test suite with smoke tests for lazy loading, comprehensions, case expressions
   - Example apps in `examples/apps/`
   - Benchmark infrastructure in `benchmarks/`

4. **Documentation**
   - `PROMPT.md` - comprehensive stdlib usability specification
   - `AGENTS.md` - development guide
   - `.agents/summary/` - architecture documentation

### Critical Gaps Identified

#### Gap 1: Host Function Registration Incomplete
**Problem**: `src/interop.rs` declares modules but doesn't register all host functions
```rust
mod enum_mod;
mod http_server;
mod io_mod;
mod map_mod;
mod path_mod;
mod string_mod;
mod system;
```

**Evidence**: Only `register_sample_functions()` is called in `HostRegistry::new()`, registering just `:identity`

**Impact**: LLMs will generate code calling `IO.puts()`, `Map.keys()`, `Enum.join()` that fails at runtime

#### Gap 2: Native Backend Parity Unclear
**Problem**: `src/c_backend/stubs.rs` likely lacks host call implementations for stdlib functions

**Impact**: `tonic compile` + `./native_binary` will fail even if `tonic run` works

#### Gap 3: Documentation-Implementation Mismatch
**Problem**: PROMPT.md specifies exact stdlib surface but implementation may not match

**Example**: PROMPT.md says `IO.ansi_red/1` should exist, but need to verify it's actually registered

#### Gap 4: Error Message Quality Unknown
**Problem**: No systematic audit of error messages for LLM-friendliness

**Impact**: When LLMs make mistakes, unclear error messages lead to longer recovery loops

#### Gap 5: Example Corpus Gaps
**Problem**: Need to verify example apps cover all stdlib modules adequately

**Impact**: LLMs learn from examples; missing coverage = lower generation quality

---

## Research Questions

### RQ1: Host Function Registration
**Question**: Which stdlib host functions are actually registered vs. which are documented?

**Method**:
1. Parse `src/interop.rs` and all `src/interop/*.rs` files
2. Extract all `host_call(:atom_name, ...)` from `src/manifest_stdlib.rs`
3. Cross-reference with `registry.register()` calls
4. Generate gap report

**Success Criteria**: 100% of documented stdlib functions are registered

### RQ2: Native Backend Parity
**Question**: Which host functions have native backend implementations?

**Method**:
1. Parse `src/c_backend/stubs.rs` for host call handlers
2. Compare with registered host functions from RQ1
3. Test `tonic compile` + execute for representative functions

**Success Criteria**: All public stdlib functions work in both interpreter and native modes

### RQ3: Error Message Audit
**Question**: Are error messages clear, actionable, and LLM-friendly?

**Method**:
1. Catalog all error paths in `src/runtime.rs`, `src/runtime_eval.rs`, `src/interop.rs`
2. Test with intentional errors (wrong arity, type mismatches, undefined functions)
3. Score messages on: specificity, actionability, line numbers, type info

**Success Criteria**: All errors include file:line, expected vs. actual, and recovery hint

### RQ4: Documentation Accuracy
**Question**: Does PROMPT.md match actual implementation?

**Method**:
1. Extract function signatures from PROMPT.md
2. Compare with `src/manifest_stdlib.rs` implementations
3. Verify arity, behavior, and edge cases

**Success Criteria**: PROMPT.md is source of truth; any discrepancies are documented

### RQ5: Example Coverage
**Question**: Do example apps demonstrate all stdlib modules?

**Method**:
1. Catalog all example apps in `examples/apps/`
2. Parse for stdlib module usage
3. Identify uncovered modules/functions

**Success Criteria**: Every stdlib module has ≥2 example apps; every function has ≥1 usage

---

## Implementation Plan

### Phase 1: Discovery & Gap Analysis (Week 1)

**Tasks**:
1. Run automated host function registration audit
2. Test native backend parity for all stdlib functions
3. Collect error messages from intentional failures
4. Parse PROMPT.md and compare with implementation
5. Catalog example app coverage

**Deliverables**:
- `autoresearch.gap-report.md` - detailed gap analysis
- `autorehost.missing-host-funcs.txt` - list of unregistered host functions
- `autoresearch.native-parity.csv` - interpreter vs. native support matrix
- `autoresearch.error-messages.md` - error message quality audit
- `autoresearch.example-coverage.md` - example app coverage report

**Automation Script**: `autoresearch.discovery.sh`

```bash
#!/bin/bash
# Automated discovery phase

echo "=== Host Function Registration Audit ==="
# Extract all host_call atoms from manifest_stdlib.rs
grep -oP 'host_call\(:[\w_]+\)' src/manifest_stdlib.rs | sort -u > /tmp/all-host-calls.txt

# Extract all registered functions from interop.rs
grep -oP 'register\("[\w_]+"' src/interop/*.rs | sort -u > /tmp/registered-funcs.txt

# Find gaps
comm -23 /tmp/all-host-calls.txt /tmp/registered-funcs.txt > autoresearch.missing-host-funcs.txt

echo "=== Native Backend Parity Test ==="
# Test each stdlib module in native mode
for module in System String Path IO List Map Enum; do
    echo "Testing $module in native mode..."
    # Create test file, compile, run, compare output
done

echo "=== Error Message Collection ==="
# Run tests with intentional errors
cargo test 2>&1 | grep -A2 "error:" > autoresearch.error-messages.md

echo "=== Example Coverage Analysis ==="
# Parse example apps for stdlib usage
find examples/apps -name "*.tn" -exec grep -l "System\.\|String\.\|Path\.\|IO\.\|List\.\|Map\.\|Enum\." {} \;
```

### Phase 2: Host Function Registration (Week 2)

**Tasks**:
1. Register all missing host functions in `src/interop.rs`
2. Implement any missing host function handlers in `src/interop/*.rs`
3. Add unit tests for each registered function
4. Verify lazy loading triggers correctly

**Files to Modify**:
- `src/interop.rs` - add `register()` calls for all stdlib functions
- `src/interop/io_mod.rs` - verify IO.puts, IO.inspect, IO.gets, IO.ansi_*
- `src/interop/map_mod.rs` - verify Map.keys, values, merge, drop, take, get, put, delete, has_key
- `src/interop/enum_mod.rs` - verify Enum.join, Enum.sort (host-backed portions)
- `tests/run_stdlib_host_funcs.rs` - new test file

**Verification**:
```bash
# Test each stdlib function
tonic run -e 'IO.puts("hello")'
tonic run -e 'Map.keys(%{a: 1, b: 2})'
tonic run -e 'Enum.join([1, 2, 3], ",")'
```

### Phase 3: Native Backend Parity (Week 3)

**Tasks**:
1. Add host call stubs in `src/c_backend/stubs.rs`
2. Implement native host function handlers
3. Test `tonic compile` + native binary execution
4. Add parity tests to test suite

**Files to Modify**:
- `src/c_backend/stubs.rs` - add host function handlers
- `src/c_backend/host_calls.rs` - implement native handlers
- `tests/native_stdlib_parity.rs` - new test file

**Verification**:
```bash
# Compile and run native binary
tonic compile examples/apps/hello-world
./hello-world
# Compare output with interpreter
tonic run examples/apps/hello-world
```

### Phase 4: Error Message Improvements (Week 4)

**Tasks**:
1. Improve error messages in `src/runtime.rs`
2. Add line number context to all errors
3. Add type information to type mismatch errors
4. Add recovery hints to common errors

**Files to Modify**:
- `src/runtime.rs` - improve error message formatting
- `src/runtime_eval.rs` - add context to eval errors
- `src/interop.rs` - improve host function error messages

**Before/After Example**:
```
Before: "undefined function"
After: "error at examples/app.tn:42: undefined function 'Foo.bar/1'. 
        Did you mean 'Bar.foo/1'? Available functions in module Foo: baz/0, qux/2"
```

### Phase 5: Documentation Sync (Week 5)

**Tasks**:
1. Update PROMPT.md to match actual implementation
2. Add inline documentation to all stdlib modules
3. Generate API reference from source
4. Add examples to documentation

**Files to Modify**:
- `PROMPT.md` - sync with implementation
- `src/manifest_stdlib.rs` - add module-level docs
- `docs/stdlib.md` - auto-generated API reference
- `examples/` - add missing example apps

**Documentation Generation**:
```bash
# Extract stdlib docs from source
python scripts/generate_stdlib_docs.py > docs/stdlib.md
```

### Phase 6: Example Corpus Expansion (Week 6)

**Tasks**:
1. Create example apps for uncovered stdlib functions
2. Add edge case examples
3. Add performance comparison examples
4. Add real-world use case examples

**New Example Apps**:
- `examples/apps/stdlib-io-demo.tn` - comprehensive IO module demo
- `examples/apps/stdlib-map-demo.tn` - Map operations showcase
- `examples/apps/stdlib-enum-demo.tn` - Enum transformations
- `examples/apps/real-world-web-server.tn` - HTTP server example
- `examples/apps/real-world-data-pipeline.tn` - Data processing example

### Phase 7: LLM Evaluation & Benchmarking (Week 7)

**Tasks**:
1. Create LLM benchmark suite
2. Run LLM code generation tests
3. Measure pass@1, pass@5, pass@10
4. Identify failure modes
5. Iterate on improvements

**Benchmark Structure**:
```
benchmarks/llm-eval/
├── tasks/
│   ├── 001-hello-world.md
│   ├── 002-list-processing.md
│   ├── 003-map-operations.md
│   ├── 004-enum-transforms.md
│   ├── 005-web-server.md
│   └── ...
├── expected_output/
│   └── ...
└── run_benchmark.sh
```

**Evaluation Metric**:
```
pass@k = 1 - ((n-k)/n * (n-k-1)/(n-1) * ... * (n-k+1)/(n-k+2))
where n = total tasks, k = number of samples
```

### Phase 8: Final Integration & Testing (Week 8)

**Tasks**:
1. Run full test suite
2. Verify all phases complete
3. Update autoresearch.md with results
4. Document lessons learned
5. Create maintenance plan

**Verification Checklist**:
- [ ] All stdlib functions registered
- [ ] All stdlib functions work in native mode
- [ ] All error messages include file:line and recovery hints
- [ ] PROMPT.md matches implementation
- [ ] All stdlib modules have ≥2 example apps
- [ ] LLM pass rate ≥95% on benchmark suite
- [ ] Full test suite passes (`cargo test`)
- [ ] No compiler warnings

---

## Automation Infrastructure

### autorehost.discovery.sh
Automated gap analysis and discovery (Phase 1)

### autorehost.benchmark-runner.sh
LLM benchmark execution and scoring (Phase 7)

### autorehost.error-collector.py
Systematic error message collection and analysis

### autorehost.doc-sync.py
Documentation vs. implementation diff checker

### autorehost.example-generator.py
Generates example apps for uncovered functions

---

## Success Metrics

### Primary Metric
**LLM Pass Rate**: Target ≥95% pass@1 on benchmark suite

### Secondary Metrics
1. **Host Function Coverage**: 100% of documented functions registered
2. **Native Parity**: 100% of stdlib functions work in native mode
3. **Error Message Quality**: 100% include file:line, 90% include recovery hints
4. **Documentation Accuracy**: 0 discrepancies between PROMPT.md and implementation
5. **Example Coverage**: Every stdlib module has ≥2 examples, every function has ≥1 usage
6. **Test Coverage**: ≥80% of stdlib code covered by tests

---

## Risk Mitigation

### Risk 1: Native Backend Complexity
**Mitigation**: Start with interpreter-only functions, document native gaps clearly, prioritize high-value functions

### Risk 2: Recursion Depth Limits
**Mitigation**: Profile pure-Tonic implementations, add tail call optimization where needed, document limits

### Risk 3: Closure Parity Across Backends
**Mitigation**: Test closures in all backends early, document differences, prioritize interpreter for closure-heavy code

### Risk 4: Performance Regressions
**Mitigation**: Benchmark before/after each phase, profile pure-Tonic vs. host-backed, optimize hot paths

---

## Timeline

| Phase | Week | Deliverables |
|-------|------|-------------|
| 1. Discovery | Week 1 | Gap analysis reports |
| 2. Host Registration | Week 2 | All stdlib functions registered |
| 3. Native Parity | Week 3 | Native backend support |
| 4. Error Messages | Week 4 | Improved diagnostics |
| 5. Documentation | Week 5 | Synced docs |
| 6. Examples | Week 6 | Complete example corpus |
| 7. LLM Benchmark | Week 7 | Pass rate measurement |
| 8. Integration | Week 8 | Final verification |

**Total**: 8 weeks to production-ready LLM-optimized Tonic

---

## Next Steps (Immediate)

1. **Run Phase 1 discovery scripts** - generate gap analysis
2. **Create `autoresearch.gap-report.md`** - document findings
3. **Prioritize missing host functions** - focus on high-impact ones first
4. **Set up LLM benchmark infrastructure** - prepare evaluation suite
5. **Schedule weekly reviews** - track progress against metrics

---

## Related Files

- `PROMPT.md` - stdlib usability specification
- `autoresearch.md` - autoresearch framework
- `autoresearch.checks.sh` - verification scripts
- `autoresearch.ideas.md` - improvement ideas
- `src/manifest_stdlib.rs` - stdlib implementations
- `src/interop.rs` - host function registry
- `benchmarks/` - benchmark infrastructure
- `examples/apps/` - example applications

---

## Appendix: Current Stdlib Surface

### IO Module (Host-Backed)
- `IO.puts/1` - print to stdout
- `IO.inspect/1` - pretty print
- `IO.gets/1` - read from stdin
- `IO.ansi_red/1`, `IO.ansi_green/1`, `IO.ansi_yellow/1`, `IO.ansi_blue/1`, `IO.ansi_reset/0` - ANSI colors

### List Module (Pure Tonic)
- `List.first/1`, `List.last/1`, `List.wrap/1`, `List.flatten/1`
- `List.zip/2`, `List.unzip/1`
- `List.delete/2`, `List.duplicate/2`, `List.insert_at/3`

### Map Module (Hybrid)
**Host-Backed**: `Map.keys/1`, `Map.values/1`, `Map.merge/2`, `Map.drop/2`, `Map.take/2`, `Map.get/3`, `Map.put/3`, `Map.delete/2`, `Map.has_key/2`
**Pure Tonic**: `Map.to_list/1`, `Map.new/0`, `Map.from_list/1`

### Enum Module (Mostly Pure Tonic)
- Aggregation: `count/1`, `sum/1`, `min/1`, `max/1`
- Transformation: `map/2`, `flat_map/2`, `sort/1`, `reverse/1`, `sort_by/2`
- Filtering: `filter/2`, `reject/2`, `take/2`, `drop/2`, `take_while/2`, `drop_while/2`
- Searching: `find/2`, `any/2`, `all/2`, `member/2`, `at/2`
- Grouping: `group_by/2`, `chunk_every/2`, `chunk_by/2`, `split/2`
- Uniqueness: `unique/1`, `uniq_by/2`, `dedup/1`
- Combination: `zip/2`, `zip_with/3`, `intersperse/2`, `join/2`
- Reduction: `reduce/3`, `scan/3`, `into/2`
- Analysis: `frequencies/1`, `count_by/2`
- Utilities: `with_index/1`, `each/2`, `min_by/2`, `max_by/2`, `map_join/3`

**Note**: `Enum.join/2` and `Enum.sort/1` use host calls for performance

---

*Last Updated*: March 20, 2026
*Author*: Rook O'Claw (Hermes Agent)
*Status*: Ready for Phase 1 execution