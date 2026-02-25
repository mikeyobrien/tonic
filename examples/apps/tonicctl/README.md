# tonicctl (pure-tonic meta-tool example)

`examples/apps/tonicctl` is a **plan-only** meta-tool example written in Tonic.

It demonstrates how a Tonic app can model:
- doctor checks
- gate sequences
- strict benchmark policy flows (interpreter + compiled)
- release dry-run checklist structure

With the addition of `System.argv()`, the tool now dispatches specific commands via CLI arguments. Because this is pure Tonic, it emits deterministic plan data instead of executing shell commands.

## Run

You can dispatch various subcommands by passing arguments to the script:

```bash
tonic run examples/apps/tonicctl doctor
tonic run examples/apps/tonicctl gates
tonic run examples/apps/tonicctl bench --strict
tonic run examples/apps/tonicctl release --dry-run
```

## Validate

```bash
tonic check examples/apps/tonicctl
```

## Compile

```bash
tonic compile examples/apps/tonicctl --out ./.tonic/build/tonicctl-plan
./.tonic/build/tonicctl-plan doctor
```

## Notes

- For real command execution, use the repository's Rust `tonicctl` binary workflow.
- Keep this example synchronized with:
  - `scripts/native-gates.sh`
  - `benchmarks/native-compiler-suite.toml`
  - `benchmarks/native-compiled-suite.toml`
  - `docs/release-checklist.md`
