# Cross-Compilation

Tonic supports cross-compilation to four target triples via the `--target` flag
on `tonic compile`.

## Supported Targets

| Triple | Description |
|--------|-------------|
| `x86_64-unknown-linux-gnu` | 64-bit Intel/AMD Linux (default on x86_64 Linux) |
| `aarch64-unknown-linux-gnu` | 64-bit ARM Linux (Raspberry Pi, AWS Graviton, etc.) |
| `x86_64-apple-darwin` | Intel macOS (default on Intel Mac) |
| `aarch64-apple-darwin` | Apple Silicon macOS (default on M1/M2/M3 Mac) |

## Usage

```sh
# Compile for host (default — auto-detected)
tonic compile src/main.tn

# Cross-compile for aarch64 Linux from an x86_64 Linux host
tonic compile src/main.tn --target aarch64-unknown-linux-gnu

# Cross-compile for Apple Silicon from x86_64 Linux (requires clang + macOS SDK)
tonic compile src/main.tn --target aarch64-apple-darwin --out ./my-arm-binary
```

## Compiler Selection

Tonic resolves compilers in this order:

1. **clang** (preferred for cross-compilation): invoked with `-target <triple>`
2. **GNU cross-compiler** (Linux targets only): e.g. `aarch64-linux-gnu-gcc`

For host compilation, native compiler detection runs in order: `clang`, `gcc`, `cc`.

## Installing Cross-Compilation Toolchains

### aarch64-unknown-linux-gnu (ARM64 Linux)

**Debian/Ubuntu** (apt):
```sh
sudo apt install gcc-aarch64-linux-gnu
# or use clang (already handles -target aarch64-unknown-linux-gnu)
sudo apt install clang
```

**Arch Linux**:
```sh
sudo pacman -S aarch64-linux-gnu-gcc
# or use clang
sudo pacman -S clang
```

**Nix/NixOS**:
```nix
pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc
# or
pkgs.clang
```

### x86_64-apple-darwin / aarch64-apple-darwin (macOS targets)

Cross-compiling to macOS requires:
- `clang` with macOS SDK support
- The macOS SDK (typically from Xcode or the Command Line Tools)

This is only fully supported when building **on macOS**. Cross-compiling to
macOS from Linux is possible but requires the macOS SDK to be available, which
has licensing constraints. Use tools like [osxcross] for Linux-to-macOS
cross-compilation.

[osxcross]: https://github.com/tpoechtrager/osxcross

## Testing Cross-Compilation

The unit tests in `src/target.rs` and `src/linker.rs` verify:

- Target triple parsing for all four supported targets
- Correct compiler flags generated for each target (`-target <triple>` for clang)
- GNU cross-compiler binary names (`aarch64-linux-gnu-gcc`, etc.)
- Default host triple detection

These tests do **not** require cross-compilers to be installed. They test flag
generation only. Actual cross-compilation requires the appropriate toolchain.

```sh
cargo test target::
cargo test linker::
```

## Constraints

- Actual cross-compiled binaries require the target toolchain to be installed.
- macOS cross-compilation from Linux requires the macOS SDK (see osxcross).
- Windows targets are not yet supported.
