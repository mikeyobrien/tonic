# Autoresearch Ideas: LLM Production Readiness

## High Priority

### 1. Audit PROMPT.md accuracy against actual implementation
- PROMPT.md is the primary document LLMs will use to understand Tonic
- Any mismatch = LLM generates code that doesn't work = failed pass rate
- Compare every stdlib claim in PROMPT.md against actual interop implementations

### 2. Create a comprehensive LLM-facing language reference
- Single-file, structured reference covering all syntax, stdlib, and idioms
- Optimized for LLM context windows (concise but complete)
- Include "what NOT to do" sections for common Elixir patterns that don't work in Tonic

### 3. Improve error messages for common LLM mistakes
- LLMs will try Elixir patterns that Tonic doesn't support
- Error messages should say "Tonic does not support X, use Y instead"
- Catalog the most common failure modes from example app development
- Preserve permissive `dynamic` arithmetic typing while enriching E2001 output: route actionable operator hints through the diagnostic layer for concrete non-numeric operands, but keep `dynamic` operands accepted so diagnostic work does not change inference behavior.

### 4. Stdlib completeness push (from PROMPT.md)
- Complete the IO, List, Map, Enum module exposure
- LLMs expect these to work if documented

### 5. Example app diversity and quality
- More examples = better LLM training signal
- Cover different domains: data processing, CLI tools, algorithms, text manipulation
- Each example should demonstrate idiomatic Tonic patterns

## Medium Priority

### 6. Add `tonic check` integration for LLM feedback loop
- LLMs can use `tonic check` to validate before running
- Ensure check catches the errors LLMs are likely to make

### 7. Create a "Tonic for Elixir developers" migration guide
- LLMs trained on Elixir need to know the differences
- What's the same, what's different, what's missing

### 8. Benchmark LLM generation quality
- Create a test harness that prompts an LLM to write Tonic programs
- Measure pass rate across different task categories
- Use this as the primary metric for improvement

## Lower Priority

### 9. REPL improvements for interactive LLM use
### 10. LSP quality for IDE-assisted LLM coding
- Use the new request-id and stream-frame substrate to add an `interrupt` op for long-running remote eval/load-file requests.

### Re-attempt Keyword module with focused integration tests
- Run 43 discarded because pure Tonic Keyword module had no focused Rust tests
- Re-implement with integration test file (like other stdlib modules) that exercises get/fetch/put/delete/merge through eval
- This would add ~10-15 focused tests and increase the metric
- Same applies to any future pure-Tonic-only modules: always add eval-based integration tests
