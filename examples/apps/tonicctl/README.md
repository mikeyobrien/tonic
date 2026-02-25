# tonicctl (pure-tonic meta-tool example)

`examples/apps/tonicctl` is a **plan-only** meta-tool example written in Tonic.

It demonstrates how a Tonic app can model:
- doctor checks
- gate sequences
- strict benchmark policy flows (interpreter + compiled)
- release dry-run checklist structure

Because this is pure Tonic, it emits deterministic plan data instead of executing shell commands.

## Run

```bash
tonic run examples/apps/tonicctl
```

## Validate

```bash
tonic check examples/apps/tonicctl
```

## Compile

```bash
tonic compile examples/apps/tonicctl --out ./.tonic/build/tonicctl-plan
./.tonic/build/tonicctl-plan
```

## Notes

- For real command execution, use the repository's Rust `tonicctl` binary workflow.
- Keep this example synchronized with:
  - `scripts/native-gates.sh`
  - `benchmarks/native-compiler-suite.toml`
  - `benchmarks/native-compiled-suite.toml`
  - `docs/release-checklist.md`
