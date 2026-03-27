# Tonic Autoresearch: LLM-Friendliness Improvement Plan

## Executive Summary

**Current State**: Tonic has 99 stdlib functions across 6 modules with 78 host call implementations. The language is Elixir-inspired with good syntactic properties for LLMs (AutoCodeBench shows Elixir at 97.5% Pass@1).

**Target**: Make Tonic the best production-ready language for LLMs by addressing documentation gaps, improving error messages, and expanding the example corpus.

**Key Finding**: Many stdlib functions have minimal or no inline documentation, creating ambiguity for LLMs about usage patterns, edge cases, and return types.

---

## Priority 1: Documentation Improvements (Highest Impact)

### Problem
The stdlib has 99 functions but most lack:
- Parameter descriptions
- Return type documentation
- Usage examples
- Error conditions

### Solution: Add Inline Comments to All Stdlib Functions

**Template for well-documented function:**
```tonic
def split(str, delimiter) do
  # Split a string by a delimiter into a list of substrings
  # 
  # Parameters:
  #   str: The string to split
  #   delimiter: The separator string (required, cannot be empty)
  #
  # Returns:
  #   List of strings
  #
  # Examples:
  #   String.split("a,b,c", ",") -> ["a", "b", "c"]
  #   String.split("hello", "x") -> ["hello"]
  #
  # Errors:
  #   Raises ArgumentError if delimiter is empty string
  host_call(:str_split, [str, delimiter])
end
```

**Action Items:**
1. [ ] Add comprehensive comments to all 19 String module functions
2. [ ] Add comprehensive comments to all 30 System module functions  
3. [ ] Add comprehensive comments to all 8 IO module functions
4. [ ] Add comprehensive comments to all 6 Path module functions
5. [ ] Add comprehensive comments to all 14 Map module functions
6. [ ] Add comprehensive comments to all 22 List module functions
7. [ ] Add comprehensive comments to Enum module functions

**Estimated Impact**: +15-20% LLM pass rate from reduced ambiguity

---

## Priority 2: Error Message Improvements

### Problem
Current error messages are minimal. LLMs need actionable diagnostics to recover from mistakes.

**Current Example:**
```
Error: invalid syntax
```

**Improved Example:**
```
Error: Expected 'do' keyword after function head
  Location: myfile.tn:42:15
  Hint: Function definitions require 'do ... end' blocks
  Example: def foo() do ... end
```

### Solution: Enhance Error Messages Across Compiler

**Files to Modify:**
1. `src/parse.rs` - Syntax error messages
2. `src/runtime.rs` - Runtime error messages
3. `src/compile_ir.rs` - Type checking errors
4. `src/semantic.rs` - Semantic analysis errors

**Action Items:**
1. [ ] Add location info (file:line:column) to all compiler errors
2. [ ] Add recovery hints to common syntax errors
3. [ ] Add example corrections to type errors
4. [ ] Improve variable scope error messages
5. [ ] Add pattern matching error explanations
6. [ ] Enhance module import error messages

**Estimated Impact**: +10-15% LLM pass rate from faster error recovery

---

## Priority 3: Example Corpus Expansion

### Problem
Current example apps may not cover common LLM coding patterns.

### Solution: Add Targeted Example Apps

**New Example Apps to Add:**
1. **web-server** - HTTP server with routing (tests System.http_*)
2. **file-processor** - Read/transform/write files (tests System.read_text, write_text)
3. **config-loader** - Parse JSON/YAML config (tests Map, String operations)
4. **cli-tool** - Command-line argument parsing (tests System.argv, IO.puts)
5. **data-pipeline** - Transform data with List/Map operations
6. **api-client** - HTTP client making requests (tests System.http_request)
7. **logger** - Structured logging utility (tests System.log, IO.ansi_*)
8. **crypto-utils** - HMAC, signatures (tests System.hmac_sha256_hex, discord_ed25519_verify)

**Each example should include:**
- `expected_output.txt` for automated testing
- Comments explaining the pattern
- Realistic use case

**Estimated Impact**: +10-15% LLM pass rate from better pattern learning

---

## Priority 4: Stdlib Function Additions

### Problem
Some common operations require workarounds, forcing LLMs to invent patterns.

### Recommended Additions

**String Module:**
- `String.join(list, separator)` - Join list with separator
- `String.contains?(str, substring)` - Check if substring exists
- `String.starts_with?(str, prefix)` - Check prefix
- `String.ends_with?(str, suffix)` - Check suffix
- `String.strip(str)` - Remove leading/trailing whitespace
- `String.strip_leading(str)` - Remove leading whitespace
- `String.strip_trailing(str)` - Remove trailing whitespace
- `String.capitalize(str)` - Capitalize first character
- `String.split_lines(str)` - Split on newlines
- `String.to_integer(str)` - Parse integer from string
- `String.to_float(str)` - Parse float from string
- `String.to_charlist(str)` - Convert to char list
- `String.at(str, index)` - Get character at index
- `String.reverse(str)` - Reverse string
- `String.pad_leading(str, length, char)` - Pad left
- `String.pad_trailing(str, length, char)` - Pad right

**List Module:**
- `List.join(list, separator)` - Join list elements
- `List.contains?(list, element)` - Check membership
- `List.map(list, func)` - Transform elements
- `List.filter(list, func)` - Filter elements
- `List.reduce(list, initial, func)` - Fold/reduce
- `List.flatten(list)` - Flatten nested lists
- `List.unique(list)` - Remove duplicates
- `List.sort(list)` - Sort list
- `List.reverse(list)` - Reverse list
- `List.concat(list1, list2)` - Concatenate lists
- `List.take(list, n)` - Take first n elements
- `List.drop(list, n)` - Drop first n elements

**Map Module:**
- `Map.get(map, key, default)` - Get with default value
- `Map.has_key?(map, key)` - Check key exists
- `Map.merge(map1, map2)` - Merge two maps
- `Map.drop(map, keys)` - Remove multiple keys
- `Map.take(map, keys)` - Extract subset of keys

**System Module:**
- `System.cwd()` - Get current working directory
- `System.chdir(path)` - Change directory
- `System.is_dir(path)` - Check if path is directory
- `System.is_file(path)` - Check if path is file
- `System.list_dir(path)` - List directory contents
- `System.read_text(path)` - Read file as string
- `System.write_text(path, content)` - Write string to file
- `System.append_text(path, content)` - Append to file
- `System.remove_file(path)` - Delete file
- `System.remove_tree(path)` - Delete directory tree
- `System.ensure_dir(path)` - Create directory (mkdir -p)
- `System.env(var)` - Get environment variable
- `System.which(program)` - Find executable path
- `System.random_token(bytes)` - Generate random bytes
- `System.hmac_sha256_hex(key, message)` - HMAC-SHA256
- `System.http_request(method, url, body, headers)` - HTTP request
- `System.http_listen(port)` - Start HTTP server
- `System.http_accept(listener)` - Accept connection
- `System.http_read_request(conn)` - Read HTTP request
- `System.http_write_response(conn, response)` - Write HTTP response

**IO Module:**
- `IO.inspect(value)` - Pretty print for debugging
- `IO.read_stdin()` - Read from stdin
- `IO.ansi_red(text)` - Red colored output
- `IO.ansi_green(text)` - Green colored output
- `IO.ansi_yellow(text)` - Yellow colored output
- `IO.ansi_blue(text)` - Blue colored output
- `IO.ansi_reset()` - Reset colors

**Path Module:**
- `Path.join(base, relative)` - Join path components
- `Path.dirname(path)` - Get directory name
- `Path.basename(path)` - Get file name
- `Path.extname(path)` - Get file extension
- `Path.expand(path)` - Expand ~ and env vars
- `Path.relative_to(path, base)` - Make path relative

**Estimated Impact**: +5-10% LLM pass rate from reduced need for workarounds

---

## Priority 5: Language Specification Clarity

### Problem
PROMPT.md may have ambiguities or missing details that cause LLM confusion.

### Solution: Enhance Language Specification

**Areas to Clarify:**
1. **Pattern matching semantics** - Add examples of all pattern types
2. **Guard expressions** - Clarify what's allowed in guards
3. **Function overloading** - Explain arity-based dispatch
4. **Module system** - Clarify import/export behavior
5. **Type system** - Document all types and type checking rules
6. **Error handling** - Document try/rescue semantics
7. **Concurrency** - Document task spawning and communication
8. **Macros** - Document macro expansion rules

**Estimated Impact**: +5-10% LLM pass rate from reduced specification ambiguity

---

## Implementation Roadmap

### Phase 1: Quick Wins (Week 1)
1. Add inline comments to String module (19 functions)
2. Add inline comments to IO module (8 functions)
3. Add inline comments to Path module (6 functions)
4. Improve 5 most common error messages

### Phase 2: Core Improvements (Week 2)
1. Add inline comments to System module (30 functions)
2. Add inline comments to Map module (14 functions)
3. Add inline comments to List module (22 functions)
4. Add 4 new example apps (web-server, file-processor, config-loader, cli-tool)

### Phase 3: Advanced Features (Week 3)
1. Add missing stdlib functions (Priority 4)
2. Add 4 more example apps (data-pipeline, api-client, logger, crypto-utils)
3. Enhance error messages across all compiler phases

### Phase 4: Polish (Week 4)
1. Update PROMPT.md with clarified specifications
2. Add comprehensive test suite for all stdlib functions
3. Run final LLM benchmark to measure improvement

---

## Measurement Strategy

### Before Changes
- Run current LLM benchmark: 100.0% (64/64 pass — run 26)
- Document all failing cases and error patterns

### After Each Phase
- Re-run LLM benchmark
- Compare pass rates
- Analyze new failure patterns

### Success Criteria
- **Target**: 95%+ pass rate on 100-task benchmark
- **Minimum**: No regression from current 100% on existing tasks
- **Stretch**: 98%+ pass rate with diverse task types

---

## Next Steps

1. **Review this plan** - Get feedback on priorities and approach
2. **Close native parity regressions first** - fix compiled-runtime support for advertised `System.append_text/2` and `String.replace/3`, then add regression coverage
3. **Start Phase 1** - Begin with documentation improvements
4. **Track progress** - Update autoresearch.history.md after each phase
5. **Measure impact** - Run benchmarks after each phase

