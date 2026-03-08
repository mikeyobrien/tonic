# Research: Self-Hosting / Bootstrap Milestones for Language Implementations

## Summary

Every serious compiler project uses a staged bootstrap ladder — not a single "self-hosting" flag. The milestones that matter are not feature launches but proof obligations: fixed-point/self-build checks, cross-backend differential tests, and ecosystem-scale regression sweeps. "Self-hosting" is widely overloaded as a term; the anti-patterns cluster around claiming it before the bootstrap chain is sound or before any parity evidence exists.

---

## Findings

### Milestone Ladder (Shared Pattern Across Go / Rust / GHC / Zig)

1. **Stage 0 — Trusted seed.** An external binary (older release, cross-language implementation, or WASM artifact) is the authoritative bootstrap seed. Milestone: seed is pinned, reproducible, and provenance-documented.
   [Rust bootstrap](https://rustc-dev-guide.rust-lang.org/building/bootstrapping/what-bootstrapping-does.html) · [GCC bootstrap](https://gcc.gnu.org/install/build.html) · [Zig 0.11 notes](https://ziglang.org/download/0.11.0/release-notes.html)

2. **Stage 1 — First native build.** Stage 0 compiles a working compiler from in-tree source. Milestone: it compiles and passes a basic unit/smoke suite.

3. **Stage 2 — Distributable compiler.** Stage 1 rebuilds to produce what gets shipped. This is Rust's `stage2`, Go's post-`make.bash` binary, GHC's normally-installed compiler. Milestone: `all.bash` / full testsuite passes; this is the first genuinely *useful* milestone gate.
   [Go source build](https://go.dev/doc/install/source) · [Rust dev guide](https://rustc-dev-guide.rust-lang.org/building/bootstrapping/what-bootstrapping-does.html)

4. **Stage 3 — Fixed-point / same-result check.** Stage 2 rebuilds again; outputs of stage 2 and stage 3 must be byte-for-byte identical. GCC treats any mismatch here as potential miscompilation. Rust makes this optional but documented. This is the strongest single-compiler correctness gate.
   [GCC 3-stage bootstrap](https://gcc.gnu.org/install/build.html) · [Rust stage3 note](https://rustc-dev-guide.rust-lang.org/building/bootstrapping/what-bootstrapping-does.html)

5. **Cross-backend differential parity.** Run the same program corpus through two independent backends (or interpreter vs compiled) and compare outputs. Mismatches indicate miscompilation. Tools: Csmith (C), YARPGen, Rustlantis (Rust across LLVM / Cranelift / Miri). Zig tracks backend parity as a percentage (e.g., x86 backend at 97% parity vs LLVM in 0.12, 98% in 0.14) and uses this as a release-quality signal. Rust CI runs a `cg_gcc` job for GCC-backend differential coverage.
   [Csmith PLDI 2011](https://users.cs.utah.edu/~regehr/papers/pldi11-preprint.pdf) · [Rustlantis OOPSLA 2024](https://research.ralfj.de/papers/2024-oopsla-rustlantis.pdf) · [Rust GCC backend tests](https://rustc-dev-guide.rust-lang.org/tests/codegen-backend-tests/cg_gcc.html) · [Zig 0.14 notes](https://ziglang.org/download/0.14.0/release-notes.html)

6. **Ecosystem / Crater sweep.** For changes with broad blast radius, run the compiler against a large corpus of real-world packages and flag regressions. Rust calls this Crater; Go uses first-class-port builders + trybots. Milestone: no new failures against the ecosystem snapshot.
   [Rust Crater](https://rustc-dev-guide.rust-lang.org/tests/crater.html) · [Go ports policy](https://go.dev/doc/install/source)

7. **Target tier / platform matrix.** New ports require CI success on a defined matrix before tier promotion. Zig formalizes this with Tier 1 requiring non-LLVM codegen, Tier 2 requiring CI build+test on every master commit.
   [Zig 0.14 release notes](https://ziglang.org/download/0.14.0/release-notes.html)

---

### Differential Testing in Detail

Differential testing is a *semantic oracle* technique: run a program through paths A and B, compare observable outputs, treat any divergence as a bug. Three uses in compiler work:

- **Cross-backend:** Same IR or source → LLVM vs GCC vs Cranelift vs interpreter; mismatches catch codegen bugs.
- **Interpreter vs native:** Interpreter is a reference oracle for well-defined programs. Zig and Rust (via Miri) use this to validate that runtime and compile-time execution semantics agree.
- **Stage N vs stage N+1:** The staged bootstrap chain itself is a differential test. GCC's 3-stage build is essentially an automated parity check between compiler generations.

The practical milestone ladder this implies:

| Milestone | What it proves |
|---|---|
| A — No crashes on corpus | Basic stability |
| B — Cross-backend / interpreter output agreement | Semantic correctness |
| C — Bootstrap stage equivalence | Codegen determinism + self-hosting soundness |

---

### Go's Specific Gates

- **Bootstrap version policy:** Go 1.N requires Go 1.(N-2, even). Go 1.24 requires ≥ 1.22.6. Explicit policy prevents silent drift.
- **`toolstash -cmp`:** Object-file identity check between old and new compiler; used to detect unintentional codegen changes during development.
- **`gccgo` parity:** Go 1.x maintained two compiler implementations (`gc` + `gccgo`); behavioral parity was an explicit correctness constraint.
  [Go 1.5 release](https://go.dev/doc/go1.5) · [Go 1.24](https://go.dev/doc/go1.24) · [toolstash](https://pkg.go.dev/golang.org/x/tools/cmd/toolstash)

---

## Anti-Patterns / Marketing Traps

These are the failure modes to watch for in announcements (and to avoid making):

| Pattern | What's wrong |
|---|---|
| **"Written in X" ≠ bootstrappable** | Compiler code can be in the language while still requiring an external toolchain to build. Go 1.5 made this explicit. |
| **"Self-hosted" during partial capability** | Zig 0.2 called out self-hosted progress while parser/formatter were incomplete. Stage-incomplete ≠ self-hosted. |
| **"Removed old implementation"** | Zig 0.11 removed the C++ implementation but still bootstraps from a WASM seed + C compiler + LLVM. Dependency didn't disappear, just changed form. |
| **Stage-label inflation** | `stage2`/`stage3` mean different things in Rust, GHC, and GCC. Cross-project comparisons are often meaningless without definitions. |
| **"Can compile itself" ≠ determinism** | Single self-build is weaker than a fixed-point check. Stage 3 byte-identity is the real bar. |
| **Omitting the trust chain** | Self-hosting does not solve trusting-trust (Reflections on Trusting Trust, Thompson 1984). Seed provenance and reproducibility are separate from self-hosting claims. |
| **Feature-launch framing for milestones** | Announcing "self-hosting" as a marketing moment before parity evidence exists. The milestone is the proof, not the announcement. |

**Litmus test for any self-hosting claim:**
1. What is the seed compiler / toolchain? Is it pinned and reproducible?
2. What stages are defined, and what artifacts do they produce?
3. Is there a fixed-point check (stage N vs stage N+1)?
4. What target(s) / platforms does this apply to?
5. What parity evidence (test pass %, differential backend) exists?
6. What ecosystem/regression coverage was run?

---

## Sources

**Kept:**
- Rust Dev Guide — bootstrapping (rustc-dev-guide.rust-lang.org) — authoritative stage definitions, differential backend tests, Crater
- GCC build docs (gcc.gnu.org/install/build.html) — canonical 3-stage bootstrap + fixed-point description
- Go source install + 1.5 / 1.24 release notes (go.dev) — bootstrap version policy, toolstash, gccgo parity
- Zig 0.11 / 0.14 release notes (ziglang.org) — WASM bootstrap, backend parity %, CI tier criteria
- Csmith PLDI 2011 — foundational differential testing paper
- Rustlantis OOPSLA 2024 — modern multi-backend differential testing with Miri
- toolstash pkg docs (pkg.go.dev) — Go object-file identity tooling

**Dropped:**
- YARPGen OOPSLA 2020 paper (users.cs.utah.edu) — relevant but overlaps with Csmith; Csmith is cleaner primary citation
- Solidity differential testing arxiv 2025 — too domain-specific / tangential
- GHC ghc.dev wiki — partially inaccessible; GHC User Guide covered the stage semantics sufficiently

## Gaps

- **Smaller / newer language projects** (Crystal, Vale, Lobster, etc.) don't have the same documented milestone culture; bootstrap practice there is largely informal.
- **Reproducible-build integration** (how projects connect Bootstrappable Builds / `bootstrappable.builds.org` to self-hosting claims) isn't well-covered in primary docs.
- **Quantitative parity thresholds** — Zig publishes backend % parity; most other projects don't set a formal numeric bar. There's no clear community norm on "good enough."
- Next step: check Zig's CI tier spec and the Bootstrappable Builds project for whether they provide explicit numeric gates or tiered milestones worth modeling against.
