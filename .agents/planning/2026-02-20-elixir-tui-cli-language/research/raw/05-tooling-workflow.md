Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/05-tooling-workflow.md` — 19.9KB.

---

**What the research covers:**

**Core thesis:** The distance between "downloaded" and "writing useful code" is the primary adoption lever. Single-binary toolchains (Gleam, Deno, Bun, uv) consistently win.

**8 areas covered:**
1. **REPL** — stateful, recoverable, with `:doc`/`:type` built-in; Deno's TypeScript-aware REPL vs Bun's speed-first approach as the key tradeoff
2. **Script runner** — hashbang support + inline dependency declarations (uv's PEP 723 model) as the 2025 frontier; `tonic run <url>` as stretch
3. **Dependency management** — Cargo model + Hex.pm-style registry; content-addressed cache; `uv`'s speed benchmark (10-100x faster)
4. **Packaging/distribution** — single static binary, curl-to-install, `tonic build` for standalone executables; cross-compilation deferred
5. **Formatter/linter** — one opinionated formatter, no config at v0; Clippy-style lint categories; `deno fmt/lint/test` as blueprint
6. **Testing** — first-class `test` blocks in syntax; `tonic test --watch`; doctests at v0.5; coverage at v1
7. **Docs** — auto-generated HTML from structured doc comments; doctests keep examples accurate; package registry search over names + doc text
8. **Extensions** — LSP first (one server covers all editors); VS Code extension + tree-sitter grammar at launch; language macros/plugins explicitly deferred post-v1

**Roadmap:** Stage 0 (run + fmt + REPL + install) → v0.1 (tests + packages + registry) → v0.5 (LSP + doctests + cookbook) → v1.0 (stable tooling contract, autocomplete, build output, coverage)

**Key deferred items:** compiler plugins/macros, cross-compilation, DAP debugger, WASM playground, monorepo/workspace support, hot reload.