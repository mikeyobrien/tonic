# Tonicctl Meta-Tool Evolution

**Goal:** Evolve `examples/apps/tonicctl` from a pure planner into an executable meta-tool for tonic, capable of managing tonic projects, running tests, formatting, and invoking the tonic compiler natively.

**Sequence:**
1. Setup and CLI parsing
2. Core compiler integration (`tonic compile <path>`)
3. Manifest and workspace management
4. Developer tools (fmt, test)
5. Executable generation and distribution
6. Testing and documentation

**Definition of Done:**
- `tonicctl` can be compiled into a standalone executable using `tonic compile examples/apps/tonicctl/main.tn`.
- The meta-tool can orchestrate tonic workflows (build, test, format) without relying on external bash scripts.
- All 10 tasks in the sequence are completed and verified against the dual strict gates policy.
- Zero references to legacy `--backend` flags exist in the meta-tool's implementation.
