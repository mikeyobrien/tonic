# Tonic text / binary / parser contract

This document defines the current contract for **runtime text** in Tonic.

Scope:

- string literals
- `System.read_text/1`
- `System.read_stdin/0`
- text returned from host-backed APIs such as `System.run/1`

The goal is contract honesty, not Elixir cosplay.

## Current contract

Runtime text is currently a **binary-shaped string value**, not a parser-ready
byte list.

That means:

- `is_binary(text)` is the supported truthy shape check today
- `is_list(text)` is false for runtime text
- list-prefix parsing like `[43, 43, 43, 10 | rest]` does **not** match runtime text
- byte-segment bitstring parsing like `<<a, b, c, d>>` does **not** match runtime text
- Tonic does **not** currently promise implicit coercion from runtime text into
  parser-ready bytes

This divergence is specific to runtime text values. Explicit byte-list literals
and explicit `<<...>>` control values can still participate in their own list
or bitstring pattern matches.

## Supported parser-ish path today

For workload-shaped text parsing, use the workload-backed `String` helpers on
runtime text instead of implicit byte/list assumptions.

Current examples that fit the supported path:

- `String.starts_with/2`
- `String.split/2`
- `String.trim/1`
- `String.trim_leading/1`
- `String.trim_trailing/1`
- `String.contains/2`
- `String.slice/3`
- `String.to_integer/1`

That is the intended near-term contract for frontmatter-ish and CLI-style text
processing.

### Supported style

```tn
text = System.read_text("page.md")

if String.starts_with(text, "+++\n") do
  String.split(text, "\n")
else
  {:error, "missing opening fence"}
end
```

### Not a supported runtime-text parser contract today

```tn
case text do
  [43, 43, 43, 10 | rest] -> {:ok, rest}
  _ -> {:error, "missing opening fence"}
end
```

```tn
case text do
  <<a, b, c, d>> -> {a, b, c, d}
  _ -> :no_match
end
```

## Execution-mode caveat

The workload-backed `String` stdlib path above currently depends on **project
mode** optional stdlib injection.

Today that means:

- `tonic run .` on a `tonic.toml` project gets the optional stdlib surface
- `tonic compile .` on a `tonic.toml` project gets the same project-mode surface
- single-file `tonic run file.tn` still does **not** receive the optional
  stdlib injection

So the current parser-friendly contract is both:

- `String`-driven, and
- project-mode only

until execution-mode behavior is intentionally unified.

## Proof in this repo

Repo-local regression coverage for the current contract lives in:

- `tests/runtime_text_parser_contract.rs`

That test proves, in both interpreter and native compiled execution, that
runtime text loaded via `System.read_text/1`:

- reports `is_binary: true`
- reports `is_list: false`
- does not match list-prefix byte parsing
- does not match bitstring byte parsing
- still works with the supported `String.starts_with/2` path

Related workload evidence remains in `tonic-sitegen-stress`, but the contract is
now defined and regression-covered locally in `tonic` itself.

## Intentionally deferred

This contract does **not** currently promise:

- Elixir-like parser ergonomics from runtime text via implicit binary matching
- automatic conversion of runtime text into byte lists
- a polished byte-oriented frontmatter parser contract for runtime text

If Tonic improves this area later, it should do so with an explicit documented
contract and parity evidence, not by quietly broadening assumptions.
