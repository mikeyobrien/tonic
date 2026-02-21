# Research: Syntax & Semantics Design for an Elixir-like CLI/TUI Language

> Track 2 — Language feature triage: what to borrow, adapt, or drop from Elixir,
> Clojure, and Babashka for a language targeting CLI/TUI scripting.

---

## Summary

Elixir, Clojure, and Babashka each bring strong primitives for a CLI/TUI language but
carry non-trivial costs rooted in their original host environments (BEAM, JVM). The
safest v0 core borrows **pattern matching**, **data-first pipelines**, and a
**tagged-tuple error convention** from Elixir, combines them with **Clojure's
namespace discipline and sequence-oriented data model**, and deliberately defers macros,
protocols, and full immutable persistent data structures until v1 — replacing them with
simpler, less powerful, and less surprising substitutes.

---

## Findings

### 1. Modules / Namespaces

**Elixir:** `defmodule Foo.Bar` creates a hierarchical atom-backed namespace. Modules are
the only unit of code organization; functions live exclusively inside them. This is
clean for library code but **verbose for scripts** — you don't want to wrap a 20-line
CLI in a module just to define three helpers.

```elixir
defmodule MyTool.Fetch do
  def run(url), do: HTTPoison.get!(url)
end
```

**Clojure:** `(ns my-tool.fetch ...)` is lighter and file-scoped. Namespaced keywords
(`::my-tool/status`) allow data to carry provenance without coupling to a class
hierarchy. Rich Hickey explicitly designed this to enable "loose coupling" and
"stateless functions that are namespaced."

**Babashka:** Inherits Clojure's ns model. Works well for multi-file scripts; `bb` resolves
requires lazily so startup stays fast even with many namespaces.

**Recommendation for v0:**
- Allow **top-level `def` and `fn` outside any module** for script-mode use.
- Provide optional `module Foo` as a grouping construct (no mandatory nesting).
- Support **namespaced symbols** (`tool/run`) for inter-module calls without mandatory
  import lists. Avoid Elixir's requirement that every call site must `alias` or spell
  the full module name.
- Avoid Clojure's `ns` form's complexity (`(:require [...] :as ... :refer [...])`) —
  it's notorious for being the most error-prone boilerplate in Clojure.

> **Avoid:** Elixir's mandatory module wrapping for all code.
> **Borrow:** Clojure's namespace-qualified symbols for data provenance.

---

### 2. Immutability

**Elixir:** All values are immutable by default. Bindings in function scope can be
re-bound (variables reassigned), but the underlying data is never mutated.

**Clojure:** Persistent data structures (Bagwell tries, hash array mapped tries) give
O(log32 N) structural sharing. This is elegant but **allocates heavily** — a problem for
tight CLI loops where you're transforming megabytes of shell output.

**For CLI/TUI specifically:** Most CLI pipelines are **linear and transient** — data
flows in, gets transformed, and is discarded. Immutability is mostly free here (no
concurrent mutation risk), but persistent data structures' overhead matters when
piping grep output through ten transformation stages.

**Practical tradeoff:** Immutability by default is worth keeping for predictability and
debuggability. But **persistent data structures (Clojure-style) are not required** to
get the benefits. Simple value semantics (copy-on-rebind, with the optimizer free to
elide copies for linear values) suffice.

**Nushell's approach** is instructive: it uses structured immutable values through
pipelines (tables, records, lists) but doesn't need full persistent data structure
machinery because values aren't long-lived across pipeline stages.
[Nushell pipelines](https://www.nushell.sh/book/pipelines.html)

**Recommendation for v0:**
- **Immutable bindings by default.** No `let mut` ceremony for the simple case; use a
  `var` keyword or `ref` cell for explicit mutation when needed.
- **No persistent data structure library in v0.** Use copy semantics with compiler
  freedom to optimize. Introduce structural sharing only if benchmarks justify it.
- Avoid Clojure's `atom`/`agent`/`ref` concurrency model — too complex for a scripting
  v0.

---

### 3. Pattern Matching

This is the single most powerful feature to borrow from Elixir. It's one of the few
language features that **compounds well** — it makes error handling, control flow, and
data destructuring all converge on one mechanism.

**Elixir's model:**
```elixir
case File.read("config.toml") do
  {:ok, contents} -> parse(contents)
  {:error, :enoent} -> default_config()
  {:error, reason}  -> raise "read failed: #{reason}"
end

# Destructuring in function heads:
def handle({:ok, val}), do: process(val)
def handle({:error, _}), do: :skip
```

**Clojure's model:** Uses `cond`, `condp`, `case`, and `destructuring let` — powerful
but **not unified**. There's no single construct that does what Elixir's `case` + match
operator does. Libraries like `core.match` add full pattern matching but it's a
dependency and not idiomatic for all Clojure code.
[core.match](https://github.com/clojure/core.match)

**Key design question: exhaustiveness checking.**
Pattern matching without exhaustiveness is just a switch statement. With exhaustiveness,
it becomes a proof obligation. For a scripting language, strict exhaustiveness checking
at compile time is probably too heavy for v0 — but a **runtime warning/error on
unmatched patterns** is the minimum bar.

**Tradeoffs in syntax:**

Option A — Elixir-style (keyword-delimited):
```
match File.read("f.toml")
  {:ok, contents} -> parse(contents)
  {:error, :enoent} -> default_config()
end
```

Option B — Haskell/ML-style (indent-sensitive):
```
match File.read("f.toml"):
  {:ok, contents} -> parse(contents)
  {:error, :enoent} -> default_config()
```

Option C — Clojure-style (expression-first, list form):
```clojure
(match (read-file "f.toml")
  {:ok contents} (parse contents)
  {:error :enoent} default-config)
```

For a CLI language, **Option A or B** is preferable — readable in a terminal diff,
writable without Paredit, and requires no parenthesis counting.

**Recommendation for v0:**
- **Full pattern matching on function heads, `match` expression, and `let` destructuring.**
- Support tuple, list, map/record, and literal patterns.
- Runtime error on unmatched patterns (no silent fall-through).
- Defer guard clauses (`when age >= 16`) to v0.1 — they're useful but add parsing and
  type-checking complexity.

> **Borrow from Elixir:** Unified pattern matching as the primary control flow
> mechanism.
> **Improve on Elixir:** Don't require `case` inside a module; match should work as a
> top-level expression anywhere.

---

### 4. Pipelines / Threading

This is where Elixir and Clojure diverge most sharply in **syntax, not semantics**.

**Elixir `|>` (pipe-first):**
```elixir
"hello world"
|> String.split()
|> Enum.map(&String.upcase/1)
|> Enum.join(", ")
# => "HELLO, WORLD"
```
The value is always inserted as the **first argument**. Clean and predictable, but
breaks for functions that expect the subject as the second or last argument — forcing
anonymous function wrappers:
```elixir
data |> (&do_something(var, &1)).()  # ugly workaround
```

**Clojure `->` (thread-first) and `->>` (thread-last):**
```clojure
(->> "hello world"
     (str/split #" ")
     (map str/upper-case)
     (str/join ", "))
```
Two macros for two common positions. More flexible but requires the programmer to
**choose the right arrow** and to know which convention each function follows —
a persistent source of bugs for Clojure beginners.

**The deeper issue:** Both approaches are workarounds for the lack of **curried
functions**. In Haskell or F#, pipelines work because every function is automatically
partially applicable. Adding currying to a dynamically typed scripting language is
possible (Janet does a form of it) but adds cognitive overhead for the shell-script
author who just wants `ls | grep foo`.

**Shell pipe metaphor:** In a CLI language, users will intuit `|>` as "pipe the output
left into the input right" — matching the Unix shell `|`. This is worth leaning into.

**Recommendation for v0:**
- **`|>` as the primary pipeline operator, inserting at position 0 (pipe-first).**
- Provide a **placeholder syntax** (`_`) for non-first positions:
  ```
  data |> join(", ", _)   # inserts data at position 1
  ```
  This is simpler than `->>` and avoids the two-arrow confusion.
- Allow bare function names in pipelines: `data |> split |> map(upcase)`.
- Do NOT add threading macros in v0 — they're a macro facility, and we're deferring
  macros.

> **Borrow from Elixir:** `|>` pipe-first operator.
> **Improve on Elixir:** Placeholder `_` for non-first argument position.
> **Avoid from Clojure:** Two separate threading macros (`->` and `->>`).

---

### 5. Protocols / Interfaces

**Elixir protocols** enable open polymorphism:
```elixir
defprotocol Printable do
  def to_string(value)
end

defimpl Printable, for: Integer do
  def to_string(n), do: Integer.to_string(n)
end
```
Powerful — any type can implement any protocol after the fact. But protocols are
**a significant runtime and compile-time mechanism** requiring code consolidation,
dispatch tables, and protocol consolidation at startup.

**Clojure protocols** are similar but backed by JVM interfaces — fast dispatch, but
tied to host. `defmulti`/`defmethod` (multi-methods) are more flexible (dispatch on
any function of the args, not just type):
```clojure
(defmulti area :shape)
(defmethod area :circle [{:keys [r]}] (* Math/PI r r))
(defmethod area :rect   [{:keys [w h]}] (* w h))
```
This is elegant for data-driven dispatch but **requires runtime multi-method
infrastructure**.

**For CLI/TUI v0:** Protocols are overkill. The primary polymorphism need in CLI code
is:
- Rendering different record types to the terminal.
- Handling different error shapes uniformly.

Both can be satisfied with **pattern matching + tagged data** without protocol dispatch.

**Recommendation for v0:**
- **No protocols in v0.**
- Use pattern matching over tagged types for all dispatch.
- Design the data model so that `{:type, ...}` tagged tuples/records handle 95% of
  polymorphism needs.
- Plan for a protocol-like mechanism in v1 if users need open extension points (plugin
  systems, TUI widget protocols).

> **Avoid from Elixir/Clojure in v0:** Full protocol dispatch.
> **Plan for v1:** An `impl Printable for MyType` style block that desugars to
> pattern-matched dispatch tables.

---

### 6. Macros

**Elixir macros** operate on the AST via `quote/unquote`:
```elixir
defmacro unless(condition, do: block) do
  quote do
    if !unquote(condition), do: unquote(block)
  end
end
```
Hygienic by default; can be made unhygienic with `var!/2`. The Elixir community's rule
of thumb is emphatic: **"Rule 1: Don't Write Macros"** (Chris McCord,
*Metaprogramming Elixir*). Macros make debugging harder, increase compile times, and
create indirection that confuses tooling.
[AppSignal: Pitfalls of Metaprogramming in Elixir](https://blog.appsignal.com/2021/11/16/pitfalls-of-metaprogramming-in-elixir.html)

**Clojure macros** are Lisp macros — syntactic transformation over s-expressions. The
uniform syntax (homoiconic) makes them more tractable than Elixir macros, but they
still carry the same debugging and tooling costs. Even seasoned Clojurists use macros
sparingly: `core.async` is the canonical example of a macro that **only works because
it was written by language experts and encapsulates an entire concurrency model**.

**For a new language:** Starting with macros is a trap. They are:
- Hard to implement correctly (hygiene, phase separation, error messages).
- Hard for users to debug (macro expansion opacity).
- Often a proxy for missing language features that should be first-class instead.

The languages that avoided early macros and thrived (Go, Rust pre-1.0) built stable,
learnable cores. Those that leaned on macros early (early CoffeeScript, many Lisp
dialects) created unmaintainable DSL jungles.

**Recommendation for v0:**
- **No user-land macros in v0.**
- Implement all "built-in syntax sugar" (`if`, `match`, `with`, pipeline) as
  first-class syntax, not macros — this forces the language to have the right
  primitives.
- Design the AST to be inspectable (as a data structure the language itself can
  manipulate) so macros can be added in v1 without changing the compiler's core.
- Consider a **template/quasi-quote mechanism** (no general macro expansion) for code
  generation use cases.

> **Avoid from both Elixir and Clojure:** User-land macros in v0.

---

### 7. Error Model

This is the most consequential design decision for a CLI language. Three approaches
dominate:

**A. Elixir's tagged tuples + `with`:**
```elixir
with {:ok, file}     <- File.read("config.toml"),
     {:ok, parsed}   <- TOML.decode(file),
     {:ok, validated} <- validate(parsed) do
  run(validated)
else
  {:error, reason} -> IO.puts("Failed: #{reason}")
end
```
Clean for sequential happy-path code. `with` is itself a macro. The `else` clause
handles all failures with another pattern match. **Weakness:** error values are
untyped — `{:error, :enoent}` and `{:error, "bad key"}` are indistinguishable to the
type system. Callers must know the shape of errors by convention.
[Elixir error handling](https://elixirschool.com/en/lessons/intermediate/error_handling)

**B. Clojure's exception model:**
Clojure uses JVM exceptions but overlays a functional convention of returning
`{:ok, val}` or `{:error, msg}` maps. `try/catch` is still available. This creates
**two competing error channels** — a source of confusion in real codebases.

**C. Railway-Oriented Programming (Rust-style Result):**
Make `Result<T, E>` a first-class type that composes via `map`, `and_then`, `or_else`.
This is what the Elixir community's ROP libraries implement on top of macros.
[ROP in Elixir (hex)](https://hexdocs.pm/rop/readme.html)

**For CLI/TUI:** Exceptions are appropriate for **truly exceptional** situations
(OOM, signal received). For **expected failures** (file not found, bad input, API
error), the Result/tagged-tuple model is cleaner because errors are **explicit in the
call graph**.

The key weakness of Elixir's model is that errors are untyped. A better design:

```
# Typed error variants as first-class values
let result = File.read("config.toml")
match result
  Ok(contents) -> parse(contents)
  Err(FileNotFound) -> default_config()
  Err(e) -> fail("unexpected: #{e}")
end
```

**Recommendation for v0:**
- **`Ok(value)` / `Err(reason)` as built-in result variants** — not a library, not a
  convention, a language primitive.
- **`?` propagation operator** (Rust-style): `let x = File.read("f") ?` — on `Err`,
  short-circuits the current function returning the error to the caller.
- **`try/rescue` for exceptions** from host OS (signals, OOM) — separate from the
  result model.
- Do NOT make `raise`/`throw` the normal error path. Reserve exceptions for truly
  unrecoverable situations.

> **Borrow from Elixir:** Explicit error values, `with`-style sequential chaining.
> **Improve on Elixir:** Typed result variants, `?` propagation to avoid `with` boilerplate.
> **Avoid from Clojure:** Dual error channels (exceptions + return maps).

---

## Recommended Minimal Coherent Core for v0

The following seven features form a self-consistent, learnable v0 that avoids the
traps identified above.

| Feature | Decision | Rationale |
|---------|----------|-----------|
| **Modules** | Optional `module Name` blocks; top-level `def` allowed | Scripts shouldn't require ceremony |
| **Namespaces** | `module/fn` qualified calls; no mandatory `alias` or `import` | Reduce boilerplate |
| **Immutability** | Bindings immutable by default; `var` for explicit mutation | Predictability without persistent-DS overhead |
| **Pattern matching** | `match` expression + function head patterns; runtime exhaustion error | Unified control flow mechanism |
| **Pipelines** | `\|>` pipe-first; `_` placeholder for position | Familiar to Unix users; avoids `->>`/`->` confusion |
| **Error model** | `Ok(v)` / `Err(e)` built-in; `?` propagation; `try/rescue` for OS exceptions | Explicit without being verbose |
| **Polymorphism** | Pattern match on tagged records; no protocol dispatch | Sufficient for v0; room to grow |

### What is explicitly deferred to v1+

- **Macros** — implement as first-class syntax, not macros; expose AST in v1.
- **Protocols/typeclasses** — tagged data + pattern matching covers v0 needs.
- **Persistent data structures** — profile first; add only if CLI workloads demand it.
- **Concurrency model** — no actors, no agents in v0; `spawn` as a built-in for
  subprocess management only.
- **Guard clauses in patterns** (`when`) — useful but adds parsing complexity.
- **Tail-call optimization** — needed for recursion-heavy code; include if the
  interpreter has a trampoline; otherwise add loop/recur in v0.1.

### Minimal syntax sketch

```
# Module is optional
module Config

def load(path):
  match File.read(path)?    # ? propagates Err upward
    Ok(text) -> TOML.parse(text)
    Err(e)   -> Err("config load failed: #{e}")
  end
end

# Top-level script (no module required)
let args = CLI.args()

match Config.load(args.config or "default.toml")
  Ok(cfg)  -> run(cfg)
  Err(msg) -> eprintln(msg) |> exit(1)
end
```

Key properties of this sketch:
- Reads like Elixir; feels lighter than Ruby.
- Pipeline `|>` is familiar to anyone with shell experience.
- Errors are explicit but not verbose — `?` does the heavy lifting.
- Pattern match is the only branching primitive; no `if/elsif/else` chains needed
  (though `if` can be syntactic sugar over `match`).

---

## Sources

**Kept:**
- [Elixir pattern matching docs](https://hexdocs.pm/elixir/pattern-matching.html) —
  canonical source on Elixir's match operator and pin operator
- [Elixir error handling (Elixir School)](https://elixirschool.com/en/lessons/intermediate/error_handling) —
  covers tagged tuples, `with`, and exception model clearly
- [Pitfalls of Metaprogramming in Elixir (AppSignal)](https://blog.appsignal.com/2021/11/16/pitfalls-of-metaprogramming-in-elixir.html) —
  concrete pitfalls of macros from practitioners
- [Elixir Metaprogramming (Serokell)](https://serokell.io/blog/elixir-metaprogramming) —
  "Rule 1: Don't Write Macros" from Chris McCord
- [Clojure Protocols](https://clojure.org/reference/protocols) —
  official rationale for protocol design decisions
- [Clojure Namespaces critique (technomancy)](https://gist.github.com/technomancy/cc0e7800878c39e4c245fc041c11df4b) —
  honest critique of Clojure's namespace complexity
- [Railway-Oriented Programming in Elixir (hex)](https://hexdocs.pm/rop/readme.html) —
  shows what ROP looks like when implemented on top of Elixir macros
- [Good and Bad Elixir (keathley.io)](https://keathley.io/blog/good-and-bad-elixir.html) —
  practitioner advice on `with` vs `case` vs exception usage
- [Babashka GitHub](https://github.com/babashka/babashka) —
  design rationale for fast-startup scripting Clojure
- [Nushell Pipelines](https://www.nushell.sh/book/pipelines.html) —
  structured data pipeline design as a CLI-specific reference point
- [Elixir pipe vs Clojure threading (Ian Rumford)](https://ianrumford.github.io//elixir/pipe/clojure/thread-first/macro/2016/07/24/writing-your-own-elixir-pipe-operator.html) —
  detailed comparison of the two pipeline approaches
- [Minimalism in PL Design (pointersgonewild)](https://pointersgonewild.com/2022/05/23/minimalism-in-programming-language-design/) —
  principled argument for small feature sets
- [Pattern matching vs dynamic dispatch (langdev.stackexchange)](https://langdev.stackexchange.com/questions/413/static-or-dynamic-dispatch-for-ad-hoc-polymorphism) —
  exhaustiveness tradeoff between closed/open type systems

**Dropped:**
- Babashka Medium/GraalVM piece — good context on startup cost but doesn't bear on
  syntax design
- Reddit threads on immutability — anecdotal; official docs and PL theory papers are
  stronger sources
- TUI framework comparisons (Ratatui, etc.) — runtime/library concerns, not language
  design

---

## Gaps

1. **Startup time model:** None of the research directly addresses how to design a
   language runtime for sub-100ms CLI startup. This is a critical constraint that
   should inform whether the language compiles to native (via LLVM/Cranelift), runs on
   a fast interpreter, or embeds in a host runtime. Suggested next step: benchmark
   Janet, Babashka, and Nushell startup times; profile the hottest path.

2. **TUI-specific primitives:** The research covers general language design. TUI-specific
   concerns (event loops, reactive state, terminal escape codes) are not addressed here.
   Suggested: investigate Ratatui's Elm-style model-update-view as a potential standard
   library design pattern.

3. **Shell interop model:** How the language handles subprocess spawning, stdin/stdout
   piping, and exit codes is undefined. Babashka's `shell` function is a good
   reference; Nushell's external command integration is another. This deserves its own
   Track 3 research item.

4. **Type inference vs dynamic dispatch:** The `Ok(v)/Err(e)` model is stronger with
   static types (Rust) but usable dynamically (Elixir convention). Whether the language
   will have type inference (Hindley-Milner style) or remain fully dynamic is the
   biggest unresolved decision that affects everything above. Deferring this choice
   longer will make it harder to retrofit.
