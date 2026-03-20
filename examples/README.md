# Tonic Examples

## Apps

Project-mode examples in `apps/`. Run with `tonic run examples/apps/<name>`.

| Example | Description | Demonstrates |
|---------|-------------|--------------|
| [json_encoder](apps/json_encoder/) | Encode Tonic data structures to JSON strings | Multi-clause pattern dispatch, guard-based type checking, recursion, string interpolation, sigils |
| [stdlib_showcase](apps/stdlib_showcase/) | Exercise stdlib surface (List, Enum, Map) | Host function calls, nested map literals, range expressions |
| [self_hosted_lexer](apps/self_hosted_lexer/) | Tokenizer for Tonic source, written in Tonic | Recursive descent, multi-module projects, pattern matching, list processing |
| [tonicctl](apps/tonicctl/) | CLI tool for project management tasks | Case dispatch, map literals, command-line argument handling |
| [word_counter](apps/word_counter/) | Read a text file and display word frequencies | File I/O, string splitting, map accumulation, for comprehensions, recursive list processing |
| [config_parser](apps/config_parser/) | Parse a key=value config file with comment/blank line handling | File I/O, String.split, String.starts_with, Map.put, recursive accumulation |
| [csv_processor](apps/csv_processor/) | Parse a CSV file, filter rows, and display a formatted table | File I/O, string splitting, map construction, recursive filtering, table formatting |
| [file_tree](apps/file_tree/) | Print a directory tree with connectors | System.list_dir, System.is_dir, Path.join, recursion, string formatting |
| [markdown_headings](apps/markdown_headings/) | Extract headings from a markdown file and display an indented TOC | String.starts_with, String.slice, String.trim, tuple construction, recursive list building, file I/O |
| [mini_template](apps/mini_template/) | Replace `{{key}}` placeholders in a template with values from a map | String.contains, String.replace, Map.get, Map.keys, recursion over key lists |
| [text_stats](apps/text_stats/) | Read a text file and compute line/word/char counts, unique words, and top-5 frequencies | String.downcase, String.length, Map.values, Enum.count, Enum.sort, for comprehensions, List.flatten |
| [path_analyzer](apps/path_analyzer/) | Walk a directory tree and display file extension frequency | System.list_files_recursive, Path.extname, Map.keys, Map.delete, guard expressions, recursive accumulation |
| [todo_manager](apps/todo_manager/) | CLI todo list with file persistence and colored status display | System.write_text, String.to_integer, IO.ansi_green, IO.ansi_red, IO.ansi_yellow, file I/O, map construction |
| [env_report](apps/env_report/) | System environment reporter with aligned key-value display | System.env, System.cwd, System.which, String.pad_trailing, IO.ansi_blue, IO.ansi_yellow, IO.ansi_green, IO.ansi_red |
| [grep_lite](apps/grep_lite/) | Recursively search files for lines matching a query string | System.list_files_recursive, System.read_text, String.contains, String.replace, IO.ansi_green, recursive file walking |
| [ini_parser](apps/ini_parser/) | Parse INI-format config files with sections into nested maps | String.starts_with, String.replace, String.trim, String.split, Map.put, Map.get, Map.keys, stateful recursive parsing |
| [log_analyzer](apps/log_analyzer/) | Parse structured log lines, filter by severity, and display colored summary stats | String.split, String.contains, Map.put, Map.get, IO.ansi_red, IO.ansi_yellow, IO.ansi_green, IO.ansi_blue, recursive filtering |
| [diff_viewer](apps/diff_viewer/) | Compare two text files line-by-line and display colored unified diff output | System.read_text, String.split, String.contains, IO.ansi_green, IO.ansi_red, IO.ansi_blue, recursive line comparison |
| [roman_numerals](apps/roman_numerals/) | Convert integers to/from Roman numerals using recursive subtraction | List-of-tuples lookup table, guard expressions (>=), recursive subtraction, String.starts_with, String.slice, String.length |
| [morse_code](apps/morse_code/) | Encode/decode text to Morse code using multi-clause character lookup | Multi-clause pattern dispatch (38+ clauses), String.slice, String.upcase, String.split, recursive encoding/decoding |
| [caesar_cipher](apps/caesar_cipher/) | Encrypt/decrypt text using Caesar cipher with configurable shift | String.slice, String.upcase, String.starts_with, rem, modular arithmetic, multi-clause guards, recursive string building |
| [number_base](apps/number_base/) | Convert integers between decimal/binary/octal/hex bases | String.pad_leading, String.reverse, String.slice, div, rem, multi-clause pattern matching, formatted table output |
| [checksum_validator](apps/checksum_validator/) | Validate checksums using Luhn algorithm and ISBN-10/ISBN-13 | Multi-module design, rem/div modular arithmetic, multi-clause case dispatch, recursive digit extraction, String.slice, IO.ansi_green |
| [sorting_demo](apps/sorting_demo/) | Bubble sort and insertion sort with step-by-step trace and Enum.sort comparison | Tuple returns for traced state, multi-clause pattern dispatch, guard expressions (>), Enum.sort, Enum.reverse, IO.ansi_green, IO.ansi_blue |
| [unit_converter](apps/unit_converter/) | Convert between temperature (C/F/K), length (m/ft/in/cm), and weight (kg/lb/oz) units | Multi-clause tuple pattern matching, integer arithmetic, parallel list traversal, case dispatch |
| [url_parser](apps/url_parser/) | Decompose URLs into scheme/host/port/path/query components | String.split, String.contains, String.slice, Map construction, tuple unpacking, recursive string search |
| [matrix_math](apps/matrix_math/) | Matrix operations: add, scale, transpose, multiply, determinant (2x2/3x3) | Nested list processing, recursive accumulation, dot product, cofactor expansion, formatted tabular output |
| [hex_dump](apps/hex_dump/) | Hex dump of text with offset/hex/ASCII columns | String.to_charlist, character-to-integer conversion, fixed-width hex formatting, chunked list processing |
| [rpn_calculator](apps/rpn_calculator/) | Reverse Polish Notation calculator with step-by-step trace | List-as-stack pattern, String.to_integer, multi-clause operator dispatch, tuple-based traced evaluation |
| [brainfuck_interpreter](apps/brainfuck_interpreter/) | Brainfuck interpreter with tape, bracket matching, and instruction batching | Map-based tape memory, character-level string iteration, bracket matching, multi-clause instruction dispatch, recursive state threading |
| [game_of_life](apps/game_of_life/) | Conway's Game of Life cellular automaton with block, blinker, and glider patterns | Nested list grid construction, boundary-checked neighbor counting, rule-based cell state transitions, generation iteration, formatted grid display |
| [levenshtein](apps/levenshtein/) | Levenshtein edit distance with DP table display and multi-pair comparison | 2D dynamic programming via nested list building, row-by-row accumulation, min-of-three comparison, character-level string indexing, formatted distance matrix |
| [huffman_coding](apps/huffman_coding/) | Huffman encoding/decoding with tree construction and prefix code generation | Tuple-based priority queue, sorted insertion, list-based binary tree (leaf=[char], node=[left,right]), recursive tree traversal for code generation, encode/decode symmetry |
| [calendar](apps/calendar/) | Monthly calendar display with Zeller's congruence day-of-week computation | Multi-step modular arithmetic (rem, div), case-based month dispatch, fixed-width day padding (String.pad_leading), list chunking for grid rows, leap year detection |
