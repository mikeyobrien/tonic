# Project Summary — 2026-02-20 Elixir-Inspired CLI Language

## What was produced

This planning pass transformed the rough idea into a concrete, implementation-ready package for a **language-first v0**.

### 1) Requirements clarification
- `[[idea-honing.md]]`

Key decisions captured:
- Focus language core before TUI.
- Rust toolchain.
- Elixir-inspired but simplified syntax.
- LLM enablement approach: provide an `AGENTS.md` compatibility guide that clearly lists all v0 differences from Elixir so agents can reliably map prior training to this language.
- Syntax compatibility only (no BEAM/OTP target).
- Static typing with inference, mostly strict, explicit `dynamic` escape hatch.
- Result-first error model (`ok/err` + `?`).
- Interpreter + on-disk compiled cache for v0.
- Performance gates: cold <=50ms, warm <=10ms, idle RSS <=30MB.
- Dual-run BDD acceptance (auto + agent-manual).

### 2) Research artifacts
Core research:
- `[[research/research-plan.md]]`
- `[[research/06-runtime-semantics-gap.md]]`
- `[[research/07-startup-memory-techniques.md]]`
- `[[research/08-toolchain-portability-gap.md]]`
- `[[research/09-terminal-portability-gap.md]]`
- `[[research/10-practitioner-signals.md]]`
- `[[research/gap-closure-summary.md]]`

Raw subagent notes:
- `[[research/raw/01-language-scope.md]]`
- `[[research/raw/02-syntax-semantics.md]]`
- `[[research/raw/03-runtime-performance.md]]`
- `[[research/raw/04-tui-cli-model.md]]`
- `[[research/raw/05-tooling-workflow.md]]`
- `[[research/raw/06-runtime-semantics-gap.md]]`
- `[[research/raw/07-startup-memory-techniques.md]]`
- `[[research/raw/08-toolchain-portability-gap.md]]`
- `[[research/raw/09-terminal-portability-gap.md]]`

### 3) Detailed design
- `[[design/detailed-design.md]]`

Highlights:
- End-to-end architecture from parser/typechecker to IR/interpreter/cache.
- Backpressure and acceptance verification as non-negotiable requirements.
- BDD dual-run model (`@auto`, `@agent-manual`, `@human-manual`) and `auto|mixed|manual` verify modes.
- Data models for acceptance criteria and evidence capture.

### 4) Implementation plan (micro-TDD)
- `[[implementation/plan.md]]`

Highlights:
- 13 incremental, demoable implementation steps.
- Red/Green/Refactor micro tasks per step.
- BDD foundation in Step 1 and dual-run mode enforcement in Step 13.

### 5) Starter templates
- `[[implementation/templates/acceptance-slice-template.yaml]]`
- `[[implementation/templates/slice-template.feature]]`

These templates provide immediate scaffolding for dual-run acceptance.

## Brief design + implementation overview

The plan chooses a pragmatic v0 architecture: **Rust runtime + typed frontend + interpreter + persistent cache**, optimized for startup and memory constraints, while preserving familiar Elixir-like coding flow. Implementation is staged to deliver working slices quickly, with strict backpressure so “done” requires both automated checks and structured manual agent evidence.

## Recommended next steps

1. Review and approve `detailed-design.md` + `implementation/plan.md`.
2. Start Step 1 and instantiate acceptance/BDD templates for `step-01`.
3. Create `AGENTS.md` early with a concise “Elixir compatibility delta” section (unsupported features, changed semantics, and idiomatic replacements) so LLM agents can code accurately from day one.
4. Stand up CI with placeholder gate checks early (even before full runtime exists).
5. Keep TUI explicitly out-of-scope until language-core gates are stable.

## Areas to watch/refine

- Type inference scope vs v0 timeline risk.
- Protocol semantics and dispatch complexity.
- Exact BDD runner implementation choice (`cucumber-rs` vs custom).
- Size/speed tradeoffs in release profile settings as benchmarks come online.

## Connections
- [[rough-idea.md]]
- [[idea-honing.md]]
- [[design/detailed-design.md]]
- [[implementation/plan.md]]
- [[implementation/templates/acceptance-slice-template.yaml]]
- [[implementation/templates/slice-template.feature]]
- [[research/research-plan.md]]
- [[research/06-runtime-semantics-gap.md]]
- [[research/07-startup-memory-techniques.md]]
- [[research/08-toolchain-portability-gap.md]]
- [[research/09-terminal-portability-gap.md]]
- [[research/10-practitioner-signals.md]]
- [[research/gap-closure-summary.md]]
