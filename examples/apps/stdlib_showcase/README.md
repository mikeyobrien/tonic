# stdlib_showcase

A tiny project-mode example that exercises the currently supported optional stdlib surface for:

- `List`
- `Enum`
- `Map`
- non-interactive `IO.inspect/1`

## Run

```bash
cargo run --bin tonic -- run examples/apps/stdlib_showcase
```

## Why this is a project example

Optional stdlib injection is still project-mode-only.

That means this example intentionally uses:

- `tonic.toml`
- `src/main.tn`
- `tonic run <project-dir>`

A plain single-file `tonic run file.tn` path does not currently receive the same optional stdlib injection.
