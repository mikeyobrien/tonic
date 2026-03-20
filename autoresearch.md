# Autoresearch: Real-World Examples for Tonic

## Objective

Create a catalog of real-world, runnable examples in `examples/` that showcase Tonic's
capabilities as a practical programming language. Each example should demonstrate idiomatic
Tonic patterns and exercise the stdlib surface honestly. Fix any language gaps that prevent
implementing real-world programs. Maintain the catalog at `examples/README.md`.

## Metrics

- **Primary**: `example_count` (count, higher is better) — number of real-world examples
  that compile, run, and produce verified-correct output (validated against expected_output.txt or expected_patterns.txt)
- **Current Best**: 32
- **Secondary**: language gaps fixed, stdlib coverage exercised

## Benchmark Command

```bash
# Count correct real-world examples (project-mode apps in examples/apps/)
count=0; fail=0
for dir in examples/apps/*/; do
  if [ -f "$dir/tonic.toml" ]; then
    actual=$(TMPDIR=/home/mobrienv/projects/tonic/.tmp cargo run --quiet --bin tonic -- run "$dir" 2>/dev/null | sed 's/\x1b\[[0-9;]*m//g') || true
    if [ -f "$dir/expected_output.txt" ]; then
      expected=$(cat "$dir/expected_output.txt")
      if [ "$actual" = "$expected" ]; then count=$((count+1)); else fail=$((fail+1)); fi
    elif [ -f "$dir/expected_patterns.txt" ]; then
      ok=true
      while IFS= read -r pat; do [ -z "$pat" ] && continue; echo "$actual" | grep -qF "$pat" || ok=false; done < "$dir/expected_patterns.txt"
      if [ "$ok" = true ]; then count=$((count+1)); else fail=$((fail+1)); fi
    else
      if [ -n "$actual" ] || TMPDIR=/home/mobrienv/projects/tonic/.tmp cargo run --quiet --bin tonic -- run "$dir" >/dev/null 2>&1; then count=$((count+1)); else fail=$((fail+1)); fi
    fi
  fi
done
echo "correct=$count failed=$fail"
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
- **Run 4 (KEEP, metric=8)**: Added csv_processor and config_parser example apps. csv_processor: parses CSV data, filters/transforms rows, outputs formatted results. config_parser: parses key=value config files with String operations and Map building. Exercises: String.split, list processing, for comprehensions, pipes, Map.put, file I/O. Hypothesis: confirmed — Tonic handles structured text parsing patterns well.
- **Run 5 (KEEP, metric=10)**: Added markdown_headings and mini_template example apps. markdown_headings: extracts headings from markdown files, displays indented TOC. mini_template: replaces {{key}} placeholders in templates with map values. Exercises: String.starts_with, String.slice, String.trim, String.contains, String.replace, Map.get, Map.keys, tuple construction, recursion. Hypothesis: confirmed — Tonic handles string-heavy text processing and template substitution patterns well.
- **Run 6 (KEEP, metric=12)**: Added text_stats and path_analyzer example apps. text_stats: text file analysis with character/word/line counting. path_analyzer: filesystem path analysis and normalization. Exercises: String operations, Path module, file I/O, map building. Hypothesis: confirmed — Tonic handles file statistics and path manipulation patterns.
- **Run 7 (KEEP, metric=14)**: Added todo_manager and env_report example apps. todo_manager: task management with file persistence. env_report: system environment inspection and reporting. Exercises: file I/O, Map operations, list processing, string formatting. Hypothesis: confirmed — Tonic handles stateful file-backed apps and environment introspection.
- **Run 8 (KEEP, metric=16)**: Added grep_lite and ini_parser example apps. grep_lite: pattern matching in text files with line-number display. ini_parser: INI-format config file parsing with section support. Exercises: String operations, file I/O, list processing, map building. Hypothesis: confirmed — Tonic handles text search and structured config parsing patterns.
- **Run 9 (KEEP, metric=18)**: Added log_analyzer and diff_viewer example apps. log_analyzer: log file analysis with pattern extraction. diff_viewer: text diff comparison. Exercises: file I/O, string processing, list operations. Hypothesis: confirmed — Tonic handles log analysis and text comparison patterns.
- **Run 10 (KEEP, metric=20)**: Added roman_numerals and morse_code example apps. roman_numerals: Roman numeral conversion. morse_code: Morse code encoding/decoding. Exercises: pattern matching, string processing, map lookups, recursion. Hypothesis: confirmed — Tonic handles encoding/decoding and lookup-table patterns.
- **Run 11 (KEEP, metric=22)**: Added caesar_cipher and number_base example apps. caesar_cipher: Caesar cipher encryption/decryption with configurable shift. number_base: integer base conversion (decimal/binary/octal/hex). Exercises: modular arithmetic, rem/div, String.pad_leading, String.reverse, multi-clause guards, formatted table output. Hypothesis: confirmed — Tonic handles arithmetic encoding and number system conversions.
- **Run 12 (KEEP, metric=24)**: Added checksum_validator and sorting_demo example apps. checksum_validator: checksum computation and validation. sorting_demo: sorting algorithm demonstrations. Exercises: arithmetic, list processing, pattern matching, recursion. Hypothesis: confirmed — Tonic handles algorithmic examples well.
- **Run 13 (KEEP, metric=24)**: Added output correctness validation for all 24 examples — 18 deterministic apps get expected_output.txt (exact stdout match, ANSI-stripped), 6 non-deterministic apps get expected_patterns.txt (required substring checks). Updated autoresearch.checks.sh to validate correctness. Metric now measures "correct output" not just "runs without crashing." Hypothesis: confirmed — all 24 examples produce correct output under strict validation.
- **Run 14 (KEEP, metric=26)**: Added unit_converter and url_parser example apps. unit_converter: temperature/length/weight conversions with formatted output. url_parser: URL component extraction and analysis. Exercises: string processing, pattern matching, arithmetic, map operations. Hypothesis: confirmed — Tonic handles unit conversion arithmetic and URL string parsing.
- **Run 15 (KEEP, metric=28)**: Added matrix_math and hex_dump example apps. matrix_math: matrix operations (add, multiply, transpose, determinant) on nested lists. hex_dump: hex dump of text with offset/hex/ASCII columns. Exercises: nested list processing, arithmetic, number formatting, fixed-width alignment, chunked list processing. Hypothesis: confirmed — Tonic handles mathematical computation on nested data and byte-level formatting.
- **Run 16 (KEEP, metric=30)**: Added brainfuck_interpreter and rpn_calculator example apps. brainfuck_interpreter: classic language interpreter with tape/memory, bracket matching, character-level dispatch. rpn_calculator: postfix arithmetic evaluator using list-as-stack pattern. Exercises: interpreter/evaluator patterns, list-as-stack, bracket matching, multi-clause pattern matching, mutable state threading. Hypothesis: confirmed — Tonic handles interpreter and stack-based evaluator patterns.
- **Run 17 (KEEP, metric=32)**: Added game_of_life and levenshtein example apps. game_of_life: Conway's Game of Life cellular automaton with 2D grid evolution, neighbor counting, rule-based state transitions. levenshtein: string edit distance with 2D dynamic programming table construction. Exercises: nested list grid operations, neighbor counting with boundary checks, rule-based cell transitions, row-by-row DP accumulation, min-of-three comparison. Hypothesis: confirmed — Tonic handles simulation and dynamic programming patterns.
