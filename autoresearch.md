# Autoresearch: Real-World Examples for Tonic

## Objective

Create a catalog of real-world, runnable examples in `examples/` that showcase Tonic's
capabilities as a practical programming language. Each example should demonstrate idiomatic
Tonic patterns and exercise the stdlib surface honestly. Fix any language gaps that prevent
implementing real-world programs. Maintain the catalog at `examples/README.md`.

## Metrics

- **Primary**: `example_count` (count, higher is better) — number of new real-world examples
  that compile, run, and produce correct output
- **Current Best**: 6
- **Secondary**: language gaps fixed, stdlib coverage exercised

## Benchmark Command

```bash
# Count runnable real-world examples (project-mode apps in examples/apps/)
count=0; fail=0
for dir in examples/apps/*/; do
  if [ -f "$dir/tonic.toml" ]; then
    if TMPDIR=/home/mobrienv/projects/tonic/.tmp cargo run --quiet --bin tonic -- run "$dir" >/dev/null 2>&1; then
      count=$((count + 1))
    else
      fail=$((fail + 1))
    fi
  fi
done
echo "runnable=$count failed=$fail"
```

## Files in Scope

- `examples/apps/*/` — project-mode examples (create new subdirectories here)
- `examples/README.md` — catalog of all examples (create this)
- `src/interop/*.rs` — stdlib host functions (may need fixes/additions)
- `src/runtime/*.rs` — interpreter (may need fixes)
- `src/manifest.rs` — stdlib injection (may need fixes)
- `stdlib/*.tn` — pure-Tonic stdlib modules (may need fixes)

## Off Limits

- `src/parser/` — no parser changes unless blocking an example
- `src/lexer/` — no lexer changes unless blocking an example
- `PARITY.md` — not updating parity tracking
- `examples/parity/` — existing parity fixtures must not change

## Constraints

- All examples must compile and run: `tonic run examples/apps/<name>` exits 0
- Existing tests must still pass: `cargo test` exits 0
- No clippy warnings: `cargo clippy --all-targets -- -D warnings` exits 0
- Examples should use only the Core-supported stdlib surface (System, String, Path, IO, List, Map, Enum)
- Each example should be a project-mode app with `tonic.toml` + `src/main.tn`
- Examples should be practical programs a user might actually want to write

## Example Ideas (prioritized)

1. **json_encoder** — encode Tonic data structures (maps, lists, strings, numbers, bools, nil) to JSON strings. Demonstrates: recursion, pattern matching, string building, type dispatch.
2. **word_counter** — read a text file, count word frequencies, display sorted results. Demonstrates: System.read_text, String.split, Map, Enum.sort, IO.puts.
3. **file_tree** — recursively list directory contents with tree-style formatting. Demonstrates: System, Path, recursion, string formatting.
4. **csv_processor** — parse CSV data, filter/transform rows, output results. Demonstrates: String.split, List, Enum, for comprehensions, pipes.
5. **config_parser** — parse a simple key=value config file. Demonstrates: String operations, Map.put, file I/O, error handling with `with`.
6. **markdown_headings** — extract and display heading hierarchy from a markdown file. Demonstrates: String.starts_with, pattern matching, list building.

## What's Been Tried

- **Run 1 (KEEP, metric=4)**: Created json_encoder example app with multi-clause guard-based type dispatch, recursive list/map encoding, and examples/README.md catalog. All 4 apps in examples/apps/ run successfully. Hypothesis: confirmed — idiomatic Tonic can express real JSON encoding.
- **Run 2 (KEEP, metric=5)**: Added word_counter example app — reads a text file, splits into words, builds frequency map with recursive accumulator, sorts by count, displays formatted results. Exercises: System.read_text, String.split, Map.get/put, Enum.sort/reverse, for comprehensions. Hypothesis: confirmed — Tonic handles file I/O + string processing + map accumulation cleanly.
- **Run 3 (KEEP, metric=6)**: Added file_tree example app with 2 new stdlib host functions (System.list_dir, System.is_dir) — recursively walks directories, prints tree-style output with connectors. Exercises: filesystem operations, recursive tree traversal, string formatting, Path.join. Hypothesis: confirmed — Tonic can express recursive directory walking with new stdlib additions.
