# Contributing to Tonic

## Setup

After cloning the repository, configure git to use the project's commit hooks:

```bash
git config core.hooksPath .githooks
```

This enables:

- **pre-commit**: Runs `cargo fmt` (auto-fixes) and `cargo clippy -- -D warnings` on staged `.rs` files
- **pre-push**: Runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`

All clippy warnings are treated as errors. Commits with warnings will be rejected.

To bypass hooks in exceptional cases (CI tooling, etc.):

```bash
SKIP_GIT_HOOKS=1 git commit ...
```

## Development

### Build and test

```bash
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

### Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add bitstring pattern matching
fix: resolve REPL continuation for do/end blocks
refactor: split parser into modules
```

### Branch naming

```
feature/descriptive-name
fix/issue-description
```

Never commit directly to `main`.
