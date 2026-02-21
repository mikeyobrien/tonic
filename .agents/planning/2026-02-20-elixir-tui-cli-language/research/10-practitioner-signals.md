# Practitioner Signals (X/Twitter) — Supplemental

> This is weak evidence compared to official docs/benchmarks, but useful directional signal.

## Signals observed

1. **Babashka startup perception is consistently “instant”** among users building scripting/CLI tooling.
2. Reports indicate very low startup on modern macOS in recent builds (single-digit ms anecdotal).
3. **Windows ConPTY pain remains active** in developer discussions and bug reports around terminal behavior.
4. Rust + musl static builds are still a common recommendation path for portable single-binary CLIs.

## How to use this

- Treat as prioritization input, not proof.
- Use it to decide what to benchmark and harden first:
  - startup latency,
  - Windows terminal behavior,
  - static build reliability.

## Source links

- https://x.com/borkdude/status/1961756941056627189
- https://x.com/oratnac/status/2014632342451544223
- https://x.com/harsh_dev8086/status/2015612311181136238
- https://x.com/rockorager/status/2015779384309776768
- https://x.com/7i/status/2015417599689961479
- https://x.com/techSage/status/2021818382501523726
- https://x.com/PThorpe92/status/1953570274928209921
- https://x.com/macktronics/status/2021719736204370378

## Connections
- [[../idea-honing.md]]
- [[06-runtime-semantics-gap.md]]
- [[08-toolchain-portability-gap.md]]
- [[09-terminal-portability-gap.md]]
- [[small-improvement-rho-dashboard]]
