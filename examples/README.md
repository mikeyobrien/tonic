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
