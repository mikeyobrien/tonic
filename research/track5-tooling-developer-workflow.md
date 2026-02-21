# Research: Track 5 — Tooling & Developer Workflow for Fast Scripting Language Adoption

## Summary

The fastest path to scripting language adoption runs through tooling, not syntax. Languages that win
(Go, Rust, Deno, Bun) converge on a single executable that covers REPL, script runner, formatter,
linter, test runner, and package manager — zero configuration out of the box, opinionated by
default. This brief covers each pillar, the trade-offs, and a v0→v1 roadmap that defers complexity
without blocking early adopters.

---

## Findings

### 1. REPL

**A fast, low-latency REPL is the first touchpoint and must feel immediate.**

The REPL is where developers form their first opinion of a language. Clojure's REPL-driven
development model demonstrates that a deeply interactive loop can compensate for gaps elsewhere —
autocomplete, inline docs, exploratory data inspection. The key properties of a good REPL are:

- **Incremental evaluation:** each expression is compiled and run in isolation; state persists
  across entries. This requires the compiler to support partial programs without panicking.
- **Stateful sessions:** variable bindings survive across lines; the REPL should feel like a
  notebook, not a calculator.
- **Error recovery:** a syntax error in one line should not kill the session.
- **Multi-line detection:** automatically detect incomplete expressions (open parens, unfinished
  blocks) and continue prompting rather than erroring.
- **History, search, bracket matching:** standard readline/libedit expectations; not optional in
  2025.

What to skip in v0: syntax highlighting inside the REPL (defer to external readline wrappers like
`rlwrap`), structured printing of complex objects (print the raw repr), and tab-completion (add in
v0.3+ once the type system is stable).

The barrier to REPL adoption is skill, not tooling — the REPL requires understanding what to type.
Good error messages and a built-in `help()` or `:doc` command reduce this dramatically.

Sources:
- [What makes a good REPL?](https://vvvvalvalval.github.io/posts/what-makes-a-good-repl.html)
- [REPL-Driven Development and Learning Velocity](https://ericnormand.substack.com/p/repl-driven-development-and-learning)
- [Clojure REPL Guidelines](https://clojure.org/guides/repl/guidelines_for_repl_aided_development)

---

### 2. Script Runner

**Ship one binary. `lang run file.ext` runs a script. `lang run` with a shebang line makes it
executable. No setup required.**

The script runner is the killer feature for scripting language adoption. The entire value prop is:
clone repo, run script, done.

Design principles from Deno/Bun/Go:

- **Single binary with no external runtime dependency.** Deno compiles scripts to a `denort`
  bundle; Bun embeds V8/JavaScriptCore with the runtime; Go links statically. The goal is
  `curl | sh` bootstrap in under 30 seconds.
- **Shebang support:** `#!/usr/bin/env lang` works out of the box. The runner detects stdin vs
  file mode automatically.
- **URL/path execution:** `lang run https://example.com/script.lang` should fetch and run directly
  (optional, but powerful for tooling scripts).
- **Zero-config script dependencies:** ideally, a script can declare its deps inline (e.g., in a
  comment header or a manifest block) and the runner resolves them automatically without a separate
  install step.
- **`lang compile`:** produce a self-contained binary from a script + its deps, targeting other
  platforms. Defers the full cross-compilation story to v1.

Bun's position: "the entire toolchain in one binary — runtime, bundler, transpiler, npm client."
Deno's: "deno compile embeds your script and dependencies into a denort binary." Both validate the
single-binary model.

Sources:
- [Deno compile](https://docs.deno.com/runtime/reference/cli/compile/)
- [Bun — A fast all-in-one JavaScript runtime](https://bun.sh/)
- [Building scripts and CLIs with Deno](https://deno.com/learn/scripts-clis)

---

### 3. Dependency Management

**Adopt the Cargo model: lockfile, workspace, reproducible, fast. Resist the pip/npm footgun
history.**

Python's history (pip, virtualenv, conda, poetry, rye, uv) is a cautionary tale of retrofitting
reproducibility. Rust's Cargo solved this on day one. For a new language, this is table stakes.

Core design decisions:

- **Single package manifest** (`tonic.toml` or equivalent). No separate lockfile format specs — the
  toolchain owns both.
- **Lockfile by default.** Every project gets a `tonic.lock`. Checked into git. Exact versions,
  content hashes, source URLs.
- **Workspace support.** Multi-package repos with shared lockfiles (Cargo workspaces, uv
  workspaces). Needed by day 60 at the latest.
- **Hermetic installs.** Deps go to a global content-addressable cache (`~/.lang/cache`), linked
  into project envs. Never pollutes system paths.
- **Inline script dependencies.** For single-file scripts (the scripting use case), support a
  comment-header format to declare deps without a manifest file:

  ```
  # deps: ["requests@2.31", "click@8.1"]
  ```

  Deno does this with import maps; uv does it with `# /// script` PEP 723 blocks.
- **Speed as a feature.** uv resolves/installs in milliseconds vs pip's seconds. This is not
  cosmetic — slow installs break CI and discourage iteration. Write the resolver in the systems
  language the runtime is written in.

What to defer to v1: private registry support, workspace dependency patching, vendoring, binary
packages (pre-built .so/.dll extensions).

Sources:
- [uv: Python packaging in Rust](https://astral.sh/blog/uv)
- [uv GitHub](https://github.com/astral-sh/uv)
- [Rye Philosophy](https://rye.astral.sh/philosophy/)

---

### 4. Packaging & Distribution

**Make publishing a one-command experience. The registry is part of the language.**

The package registry is infrastructure you control. Don't outsource this to GitHub releases or a
community-run server in early days.

Key lessons from npm/crates.io/PyPI:

- **Official registry from day one.** `lang publish` uploads to `registry.lang.dev`. Namespacing
  by `author/package` (like crates.io) prevents squatting better than flat namespaces (npm).
- **Provenance by default.** npm provenance (generally available 2023) links packages to their
  source repo and CI build via Sigstore. Bake this in from the start — retroactively adding it to
  npm was painful. Every published package should include a signed SLSA provenance attestation.
- **`lang install -g pkg` for global tools.** Scripts and CLIs can be installed to `~/.lang/bin`
  and added to PATH. No virtualenv dance.
- **Compile to single binary as a distribution primitive.** `lang compile` targets a self-contained
  executable for Linux/macOS/Windows. This is Deno's killer distribution story. Users who don't
  have the runtime installed can still run your tool.
- **SemVer enforcement in the registry** (reject breaking changes without major bump) — optional
  but trust-building.

What to defer: private registries, org namespaces, billing, mirror protocol.

Sources:
- [npm Security 2025: Provenance and Sigstore](https://dev.to/dataformathub/npm-security-2025-why-provenance-and-sigstore-change-everything-2m7j)
- [Introducing npm package provenance](https://github.blog/security/supply-chain-security/introducing-npm-package-provenance/)
- [Deno compile executables](https://deno.com/blog/deno-compile-executable-programs)

---

### 5. Formatter & Linter

**One tool, zero configuration, opinionated defaults. Format on save should just work.**

The lesson from gofmt, Prettier, Black, and Ruff: **a formatter wins by being non-negotiable**.
The moment the formatter has configurable indent width, the ecosystem splits into style factions. Go
made gofmt mandatory and tab-indented Go is universal. Black does the same for Python.

Architecture:

- **Single binary subcommand:** `lang fmt` formats; `lang fmt --check` fails CI if unformatted.
  `lang lint` surfaces warnings; `lang lint --fix` auto-fixes safe rules.
- **Integrated AST.** The formatter operates on the same parse tree as the compiler. This means
  formatting is always syntactically valid and round-trips cleanly. No separate parser to maintain.
- **Speed from the same runtime.** Ruff formats 10-100× faster than Black/Flake8 by being written
  in Rust with the same AST as the linter. For a new language, this comes free — you're writing
  everything in the same system.
- **Lint rules in the standard library of rules.** Core rules ship with the tool; community rules
  are plugins (post-v1). Rules should have:
  - A stable ID (`L001`, `E042`)
  - An auto-fix flag
  - A `# lang:disable` inline suppression
- **Editor integration:** formatter and linter are exposed as LSP diagnostics and formatting
  providers. No separate plugin required.

What to defer: custom rule authoring in the scripting language itself, multi-language support,
semantic lint rules that require type inference.

Sources:
- [Ruff](https://docs.astral.sh/ruff/)
- [The Ruff Formatter: Black-compatible Python formatter](https://astral.sh/blog/the-ruff-formatter)
- [Ruff GitHub](https://github.com/astral-sh/ruff)

---

### 6. Testing Strategy

**Built-in test runner, doctest support, snapshot tests. No required external deps.**

Go's `testing` package and Rust's `#[test]` attribute demonstrate that built-in testing has
compounding adoption effects: every stdlib function ships with tests, tutorials use `go test`, and
CI is two lines.

Design:

- **`lang test`** discovers test files by convention (`*_test.lang` or `test_*.lang`).
  Runs in parallel by default. Reports pass/fail/skip with timings.
- **Assertion style:** prefer `expect(a).to_equal(b)` or simple `assert a == b` over xUnit
  class hierarchies. Beginners don't need `TestCase`.
- **Doctest:** code blocks in doc comments are extracted and run as tests. This is the single
  most effective forcing function for keeping docs accurate. Python's `doctest`, Rust's `///`
  examples. Implement early.
- **Snapshot testing:** `expect(output).to_match_snapshot()` — on first run, saves the output;
  on subsequent runs, fails on diff. Invaluable for CLI tools and data transformation scripts.
  Jest's snapshot model is a reference implementation.
- **Test coverage:** `lang test --coverage` produces an HTML report or lcov data. Defer to v0.5.
- **Mocking:** defer entirely to v1 or community libraries. Built-in mocking is notoriously
  hard to get right and adds significant API surface.

Sources:
- [Python Testing Frameworks](https://testgrid.io/blog/python-testing-framework/)
- [JavaScript Unit Testing 2024](https://raygun.com/blog/javascript-unit-testing-frameworks/)

---

### 7. Docs & Discoverability

**Rustdoc-class generated docs + a playground + a searchable stdlib reference.**

The discoverability problem is real: Rust's own forum has threads about how hard it is to find
methods on types even with great docs. Design around this from the start.

Pillars:

- **`lang doc`** generates HTML documentation from doc comments. Outputs to `./docs/` or serves
  locally at `localhost:8080`. Mirrors what rustdoc does, but should be faster and generate more
  navigable output.
- **Doc comment standard:** `/// Single line` and `/** block */` with Markdown body, `@param`,
  `@returns`, `@example` tags. The formatter enforces doc comments on public symbols (lint rule,
  not hard error in v0).
- **Inline `--help` in the REPL:** `:doc functionName` prints the doc comment + signature
  immediately. Zero context switch.
- **Online playground:** a browser-based editor + runner at `play.lang.dev`. This is the single
  highest-ROI investment for adoption. People share snippets, tutorials link to live examples,
  StackOverflow answers come with runnable code. Glitch/RunKit/Rextester show the pattern;
  Rust Playground is the gold standard.
- **Standard library reference:** every stdlib symbol is documented with at least one example.
  This is a policy, not a feature.
- **Search:** docs site has instant fuzzy search over stdlib symbols, types, and package docs.
  Algolia DocSearch is free for OSS.

What to defer: versioned doc hosting per package (crates.io/docs.rs split model), API stability
annotations, deprecation timeline tooling.

Sources:
- [Rust Documentation](https://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/book/first-edition/documentation.html)
- [What makes Rust docs special?](https://www.reddit.com/r/rust/comments/v6rc04/what_is_it_that_makes_rust_documentation_so_special/)
- [Zig Learn](https://ziglang.org/learn/)

---

### 8. Extension & Plugin Story

**WASM-first plugin model. Host language is irrelevant; isolation is free.**

The WebAssembly Component Model (WASI Preview 2) has emerged as the correct long-term answer for
language plugins: any language that compiles to WASM can write a plugin; the host sandbox provides
isolation; near-native performance.

Design:

- **Phase 1 (v0):** no plugin system. Extensions are just libraries. This is not a cop-out —
  it forces the stdlib to be sufficient and avoids premature API lock-in.
- **Phase 2 (v0.8):** native FFI for calling into C/Rust from scripts. `lang ffi`. This covers
  the "I need to call libsomething" use case without designing a whole plugin system.
- **Phase 3 (v1):** WASM plugin model. The linter, formatter, test runner, and doc generator
  accept WASM plugins compiled to the Component Model interface. This matches the moonrepo,
  Extism, and Envoy WASM extension patterns. Plugin authors can use any language.
  - Plugins are distributed as packages in the registry with a `plugin = true` manifest flag.
  - The runner sandboxes plugins (no filesystem/network by default; must explicitly request
    capabilities via WASI).

What to defer: a meta-programming / macro system exposed to plugins, JIT-compiled native extensions
(too much ABI surface), IDE-specific plugin packaging.

Sources:
- [Building Native Plugin Systems with WebAssembly](https://tartanllama.xyz/posts/wasm-plugins/)
- [WebAssembly Components - InfoQ](https://www.infoq.com/presentations/webassembly-extensions/)
- [WASM plugins (moonrepo)](https://moonrepo.dev/docs/guides/wasm-plugins)

---

### 9. IDE & LSP Integration

**LSP from day one. Editor-specific plugins are multipliers, not prerequisites.**

The Language Server Protocol means you write one server and get VS Code, Neovim, Helix, Zed,
IntelliJ integration for free.

- **LSP server** (`lang-server`) ships with the toolchain binary as a subcommand (`lang lsp`).
  Implements at minimum: go-to-definition, hover (type + doc), diagnostics (compiler errors +
  lint), formatting (delegates to `lang fmt`), completion (symbols in scope + stdlib).
- **VS Code extension** on the marketplace — even a thin wrapper around the LSP is enough.
  This is the highest-traffic first-install path.
- **Helix/Neovim config snippet** in the docs. These communities are early adopters of new
  languages and generate disproportionate buzz.

Sources:
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
- [LSP VS Code Extension Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide)

---

## Staged Roadmap: v0 → v1

### v0.1 — The Walking Skeleton
_Goal: a developer can write, run, and share a script in under 5 minutes._

| Deliverable | Notes |
|-------------|-------|
| `lang run file.ext` | Script runner, shebang support |
| `lang repl` | Basic REPL with history, multiline detection |
| `lang fmt` | Opinionated formatter, zero config |
| Minimal stdlib | I/O, strings, collections, math |
| Error messages | Human-readable, with line/col |
| Install script | `curl | sh` — single binary, all platforms |

**Defer everything else.**

---

### v0.3 — The Collaboration Layer
_Goal: developers can share and depend on each other's code._

| Deliverable | Notes |
|-------------|-------|
| `tonic.toml` manifest + lockfile | SemVer, hash-pinned |
| `lang add / remove / install` | Fast dep resolution, global cache |
| `lang publish` | Uploads to official registry |
| `lang test` | Test runner, parallel, doctest support |
| `lang lint` | Core lint rules with auto-fix |
| LSP server (basic) | go-to-def, hover, diagnostics |
| VS Code extension | Thin LSP wrapper on marketplace |

---

### v0.5 — The Productivity Boost
_Goal: daily-driver experience; CI/CD integration is painless._

| Deliverable | Notes |
|-------------|-------|
| Workspace support | Multi-package repos |
| Inline script deps | `# deps:` header syntax |
| `lang doc` | HTML doc generation from comments |
| Snapshot testing | `to_match_snapshot()` |
| `lang compile` | Self-contained binary output |
| Coverage reporting | `lang test --coverage` |
| Online playground | `play.lang.dev` |
| C FFI (`lang ffi`) | Call into C libraries |

---

### v0.8 — The Ecosystem Foundation
_Goal: the community can extend the toolchain and own their libraries._

| Deliverable | Notes |
|-------------|-------|
| LSP: completions + rename | Requires stable type system |
| Private registry support | For orgs and internal packages |
| Package signing (Sigstore) | Provenance attestations on publish |
| `lang upgrade` | Dependency updates with conflict detection |
| Formatter plugins (alpha) | WASM-based custom rules |
| Debugger (DAP) | Debug Adapter Protocol; breakpoints in VS Code |

---

### v1.0 — The Commitment
_Goal: API stability guarantee; production-grade signal to enterprises._

| Deliverable | Notes |
|-------------|-------|
| Stable API surface (semver contract) | No breaking changes without major version |
| WASM plugin system | Full Component Model; linter/formatter/test plugins |
| Cross-compilation | `lang compile --target windows-x86_64` from Linux |
| Versioned doc hosting | Per-package, per-version docs at registry |
| Debugger (stable) | Breakpoints, watch expressions, REPL in debug context |
| Security audit | Third-party audit of resolver, registry, runtime |

---

### What to Defer Past v1

- **Macro / meta-programming system exposed to plugins** — hard to get right, easy to create
  ecosystem fragmentation. Observe what Julia's macros and Rust's proc-macros actually needed.
- **Binary package distribution** (.so / .dll pre-built extensions) — supply chain risk;
  WASM plugins subsume most of this.
- **Versioned doc hosting per package** — needs registry infrastructure; fine to run docs.lang.dev
  with latest only until the ecosystem stabilizes.
- **JIT compilation** — orthogonal to tooling; a compiler/runtime concern.
- **Org namespaces, billing, access control in registry** — only needed when enterprise adoption
  is material.
- **IDE-native plugins** (JetBrains, Xcode) — LSP covers 90% of the need; native plugins are
  maintenance sinks.

---

## Sources

### Kept
- [What makes a good REPL?](https://vvvvalvalval.github.io/posts/what-makes-a-good-repl.html) — direct analysis of REPL design properties
- [REPL-Driven Development](https://ericnormand.substack.com/p/repl-driven-development-and-learning) — adoption barriers and skill ceiling
- [Clojure REPL Guidelines](https://clojure.org/guides/repl/guidelines_for_repl_aided_development) — practical REPL workflow patterns
- [Deno compile docs](https://docs.deno.com/runtime/reference/cli/compile/) — authoritative on single-binary distribution
- [Deno compile blog](https://deno.com/blog/deno-compile-executable-programs) — design rationale
- [Bun homepage](https://bun.sh/) — all-in-one toolchain reference
- [uv blog (Astral)](https://astral.sh/blog/uv) — Cargo-for-Python design rationale
- [uv GitHub](https://github.com/astral-sh/uv) — implementation reference
- [Rye Philosophy](https://rye.astral.sh/philosophy/) — project management design philosophy
- [Ruff docs](https://docs.astral.sh/ruff/) — unified linter+formatter reference
- [Ruff Formatter blog](https://astral.sh/blog/the-ruff-formatter) — zero-config formatter design
- [npm provenance blog](https://github.blog/security/supply-chain-security/introducing-npm-package-provenance/) — supply chain signing model
- [npm provenance 2025](https://dev.to/dataformathub/npm-security-2025-why-provenance-and-sigstore-change-everything-2m7j) — current state of sigstore adoption
- [LSP spec](https://microsoft.github.io/language-server-protocol/) — authoritative protocol definition
- [LSP VS Code guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) — integration pattern
- [WASM plugin systems (Sy Brand)](https://tartanllama.xyz/posts/wasm-plugins/) — component model implementation guide
- [Envoy WASM case study](https://eli.thegreenplace.net/2023/plugins-case-study-envoy-wasm-extensions/) — real-world WASM plugin patterns
- [moonrepo WASM plugins](https://moonrepo.dev/docs/guides/wasm-plugins/) — Rust+WASM plugin reference

### Dropped
- [Top 50 Programming Languages 2025](https://www.testdevlab.com/blog/top-50-programming-languages-in-2025) — SEO listicle, no depth
- [Wikipedia: Lint software](https://en.wikipedia.org/wiki/Lint_(software)) — too general
- [Wikipedia: LSP](https://en.wikipedia.org/wiki/Language_Server_Protocol) — superseded by official spec
- [Python has too many package managers](https://dublog.net/blog/so-many-python-package-managers/) — informative but downstream of uv blog
- [State of Kotlin Scripting 2024](https://blog.jetbrains.com/kotlin/2024/11/state-of-kotlin-scripting-2024/) — JVM-specific constraints don't transfer

---

## Gaps

1. **Inline script dependency UX benchmarks.** PEP 723 (`# /// script` blocks) is the most
   recent standard attempt at this, but adoption data is thin. Would be worth a follow-up search
   on `uv run --script` and `deno` import map UX feedback.
2. **Registry economics and governance.** How to fund, operate, and moderate a package registry
   long-term (crates.io is Rust Foundation-funded; PyPI is PSF + OTF grants). No clean model
   exists for a new language pre-community.
3. **Debugger (DAP) design specifics.** The Debug Adapter Protocol is well-documented but
   implementation guides for embedded/scripting runtimes are sparse. Worth a dedicated research
   pass when debugging is scheduled.
4. **First-week onboarding funnel data.** Which tooling gaps cause the most drop-off (REPL
   confusion? Package install failure? Missing LSP?) — no good public data exists outside of
   large ecosystems. Early user interviews would fill this.
