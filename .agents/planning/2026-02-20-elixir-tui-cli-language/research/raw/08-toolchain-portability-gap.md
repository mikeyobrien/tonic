Written to `08-toolchain-portability-gap.md` (~18 KB). Here's what's in it:

**Structure:**
- **5 toolchains evaluated** across 7 dimensions each: Rust+Cargo+cross, Zig, C/Clang/GCC, GraalVM native-image, JVM-hosted
- **Comparison matrix** — visual 5×8 grid with star ratings for quick scanning
- **Concrete numbers** — binary sizes, startup latencies, Zig version (0.14, March 2025)

**Key conclusions:**

**v0 → Rust + Cargo + `cross`**
- Static musl binaries land at ~1–3 MB stripped with the right Cargo profile flags
- `cross` (Docker-based) handles Linux/macOS/Windows/ARM without manual C toolchain management
- Memory safety matters specifically for runtime/GC/arena data structures
- Named risks: borrow-checker friction on AST graphs (mitigated by `Box<dyn>`/`Rc<RefCell>`), C FFI deps (avoid them), compile time (sccache)

**v1 → Evaluate Zig post-1.0**
- Zig's cross-compilation story is genuinely better — no separate cross toolchain, ~6 month breaking-change cadence is the only blocker
- Estimated 1.0: 2026–2027, uncertain

**GraalVM and JVM: ruled out** — 10–50 MB binaries, no cross-compilation in native-image, unacceptable startup on JVM without native-image.

**Gaps flagged:** Windows cross-compile from Linux (validate early), WASM target depth, macOS codesigning/notarization.