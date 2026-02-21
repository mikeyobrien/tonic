# Research Plan (Comprehensive)

## Objective
Design an Elixir-syntax language for high-performance CLI/TUI workloads, inspired by Babashka/Clojure, with strong portability and practical tooling.

## Workstreams

1. **Language scope + success criteria**
2. **Syntax + semantics core**
3. **Runtime architecture + performance**
4. **CLI/TUI execution model**
5. **Tooling + developer workflow**
6. **Risks + architecture decision matrix**
7. **Gap closure: runtime semantics for startup/memory**
8. **Gap closure: startup/memory implementation techniques**
9. **Gap closure: battle-tested toolchain + portability**
10. **Gap closure: terminal compatibility matrix**

## Methods

- Parallel subagent research per workstream.
- External evidence via web search and practitioner signal checks.
- Consolidation into decision-oriented markdown docs with references.

## Status snapshot

- Initial tracks completed in raw notes under `research/raw/`.
- Gap-closure docs completed:
  - [[06-runtime-semantics-gap.md]]
  - [[07-startup-memory-techniques.md]]
  - [[08-toolchain-portability-gap.md]]
  - [[09-terminal-portability-gap.md]]
  - [[10-practitioner-signals.md]]
  - [[gap-closure-summary.md]]

## Next checkpoint criteria

Proceed to design once we agree on:

1. v0 semantics profile (what is explicitly out of scope).
2. v0 runtime architecture (interpreter + cache strategy).
3. v0 toolchain and release matrix.
4. benchmark gates that define success/failure.

## Connections
- [[../idea-honing.md]]
- [[06-runtime-semantics-gap.md]]
- [[07-startup-memory-techniques.md]]
- [[08-toolchain-portability-gap.md]]
- [[09-terminal-portability-gap.md]]
- [[10-practitioner-signals.md]]
- [[gap-closure-summary.md]]
- [[small-improvement-rho-dashboard]]
