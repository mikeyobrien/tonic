Written to `/home/mobrienv/projects/tonic/.agents/planning/2026-02-20-elixir-tui-cli-language/research/raw/02-syntax-semantics.md` (22KB, ~550 lines).

---

**What the research covers and key calls:**

**Borrow from Elixir:**
- Pattern matching as a first-class dispatch form — but decouple it from `=` (binding-only `=`, explicit `match` form avoids surprise crashes)
- `{:ok}|{:error}` error convention — but upgrade it to `ok(v)/err(r)` constructors with `?` propagation (Rust ergonomics, less `with` boilerplate)
- Single-dispatch protocols — proven, fast, open for extension

**Avoid from Elixir:**
- `use MyModule` — explicitly absent; it's an implicit code injection footgun that even Elixir's own docs flag as an anti-pattern
- Pipe-as-macro — `|>` should be a language-level special form, not a macro, so it's reflectable/toolable
- Exceptions in the happy path — one `panic` for fatal errors, no class hierarchy

**Borrow from Clojure/Babashka:**
- Explicit `:as` aliasing in namespace declarations — dependency graphs stay statically analyzable
- Threading position flexibility — but via `_` placeholder on a single `|>` rather than three separate macros (`->`, `->>`, `as->`)
- "stdlib has zero macros" discipline — macros exist as a power feature but core control flow is special forms

**v0 minimal core:** flat namespaces + pattern matching + pipe-first with `_` placeholder + single-dispatch protocols + `ok/err/?` error model + one mutable `atom` cell. Everything else (supervisors, multimethods, STM, type system, macro DSLs) is explicitly deferred.

The **gaps section** flags three real open questions that need resolution before finalizing: heterogeneous error types with `?` propagation, TUI reactive state, and module circular dependency policy.