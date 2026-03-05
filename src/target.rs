/// Target triple support for cross-compilation.
///
/// Supported targets:
/// - `x86_64-unknown-linux-gnu`
/// - `aarch64-unknown-linux-gnu`
/// - `x86_64-apple-darwin`
/// - `aarch64-apple-darwin`

/// A parsed target triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetTriple {
    pub(crate) arch: Arch,
    pub(crate) os: Os,
    /// The canonical string form of the triple.
    canonical: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Os {
    Linux,
    MacOs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetParseError {
    pub(crate) message: String,
}

impl std::fmt::Display for TargetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TargetParseError {}

impl TargetTriple {
    /// Parse a target triple string.
    ///
    /// Accepts the four supported triples:
    /// - `x86_64-unknown-linux-gnu`
    /// - `aarch64-unknown-linux-gnu`
    /// - `x86_64-apple-darwin`
    /// - `aarch64-apple-darwin`
    pub(crate) fn parse(s: &str) -> Result<Self, TargetParseError> {
        match s {
            "x86_64-unknown-linux-gnu" => Ok(Self {
                arch: Arch::X86_64,
                os: Os::Linux,
                canonical: s.to_string(),
            }),
            "aarch64-unknown-linux-gnu" => Ok(Self {
                arch: Arch::Aarch64,
                os: Os::Linux,
                canonical: s.to_string(),
            }),
            "x86_64-apple-darwin" => Ok(Self {
                arch: Arch::X86_64,
                os: Os::MacOs,
                canonical: s.to_string(),
            }),
            "aarch64-apple-darwin" => Ok(Self {
                arch: Arch::Aarch64,
                os: Os::MacOs,
                canonical: s.to_string(),
            }),
            other => Err(TargetParseError {
                message: format!(
                    "unsupported target triple '{}'; supported targets: \
                     x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, \
                     x86_64-apple-darwin, aarch64-apple-darwin",
                    other
                ),
            }),
        }
    }

    /// Return the canonical string form of the triple.
    pub(crate) fn as_str(&self) -> &str {
        &self.canonical
    }

    /// Detect the host target triple at runtime.
    ///
    /// Falls back to the compile-time detected host if `TARGET` env var is unset.
    pub(crate) fn host() -> Self {
        // CARGO_CFG_TARGET_ARCH / cfg! macros are compile-time; use them.
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            Self {
                arch: Arch::X86_64,
                os: Os::Linux,
                canonical: "x86_64-unknown-linux-gnu".to_string(),
            }
        }
        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        {
            Self {
                arch: Arch::Aarch64,
                os: Os::Linux,
                canonical: "aarch64-unknown-linux-gnu".to_string(),
            }
        }
        #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
        {
            Self {
                arch: Arch::X86_64,
                os: Os::MacOs,
                canonical: "x86_64-apple-darwin".to_string(),
            }
        }
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        {
            Self {
                arch: Arch::Aarch64,
                os: Os::MacOs,
                canonical: "aarch64-apple-darwin".to_string(),
            }
        }
        #[cfg(not(any(
            all(target_arch = "x86_64", target_os = "linux"),
            all(target_arch = "aarch64", target_os = "linux"),
            all(target_arch = "x86_64", target_os = "macos"),
            all(target_arch = "aarch64", target_os = "macos"),
        )))]
        {
            // Unknown host — fall back to x86_64 linux as a safe default
            Self {
                arch: Arch::X86_64,
                os: Os::Linux,
                canonical: "x86_64-unknown-linux-gnu".to_string(),
            }
        }
    }

    /// Whether this is the host target (i.e. not cross-compiling).
    pub(crate) fn is_host(&self) -> bool {
        *self == Self::host()
    }

    /// Return the C cross-compiler binary name for this target.
    ///
    /// For host targets, returns `None` — the caller should use normal compiler
    /// detection (`clang`, `gcc`, `cc`).
    ///
    /// For cross targets:
    /// - Prefers a clang-style invocation with `-target <triple>` if `clang`
    ///   is available (returned as `None` with extra flags applied at link time).
    /// - Otherwise returns a gcc cross-compiler binary name prefix.
    pub(crate) fn cross_compiler_binary(&self) -> Option<String> {
        if self.is_host() {
            return None;
        }
        // Return the GNU cross-compiler prefix binary name.
        // Callers may also try clang with -target <triple>.
        match (&self.arch, &self.os) {
            (Arch::Aarch64, Os::Linux) => Some("aarch64-linux-gnu-gcc".to_string()),
            (Arch::X86_64, Os::Linux) => Some("x86_64-linux-gnu-gcc".to_string()),
            // macOS cross-compilation via clang only (see compiler_flags)
            (Arch::X86_64, Os::MacOs) | (Arch::Aarch64, Os::MacOs) => None,
        }
    }

    /// Return extra compiler flags needed to target this triple.
    ///
    /// For clang, pass `-target <triple>`.
    /// For native gcc, no extra flags are needed (binary prefix handles it).
    pub(crate) fn clang_target_flags(&self) -> Vec<String> {
        vec!["-target".to_string(), self.canonical.clone()]
    }

    /// Return the preferred output executable extension for this target.
    ///
    /// All four supported targets produce Unix ELF/Mach-O binaries with no
    /// extension.
    #[allow(dead_code)] // Used by linker for output naming
    pub(crate) fn exe_extension(&self) -> &'static str {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_four_supported_targets() {
        let triples = [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        ];
        for triple in triples {
            let result = TargetTriple::parse(triple);
            assert!(result.is_ok(), "expected Ok for '{triple}'");
            assert_eq!(result.unwrap().as_str(), triple);
        }
    }

    #[test]
    fn parse_unsupported_target_returns_error() {
        let result = TargetTriple::parse("wasm32-unknown-unknown");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("unsupported target triple"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn parse_arch_and_os_x86_64_linux() {
        let t = TargetTriple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(t.arch, Arch::X86_64);
        assert_eq!(t.os, Os::Linux);
    }

    #[test]
    fn parse_arch_and_os_aarch64_linux() {
        let t = TargetTriple::parse("aarch64-unknown-linux-gnu").unwrap();
        assert_eq!(t.arch, Arch::Aarch64);
        assert_eq!(t.os, Os::Linux);
    }

    #[test]
    fn parse_arch_and_os_x86_64_macos() {
        let t = TargetTriple::parse("x86_64-apple-darwin").unwrap();
        assert_eq!(t.arch, Arch::X86_64);
        assert_eq!(t.os, Os::MacOs);
    }

    #[test]
    fn parse_arch_and_os_aarch64_macos() {
        let t = TargetTriple::parse("aarch64-apple-darwin").unwrap();
        assert_eq!(t.arch, Arch::Aarch64);
        assert_eq!(t.os, Os::MacOs);
    }

    #[test]
    fn host_triple_returns_valid_target() {
        let host = TargetTriple::host();
        // Must round-trip through parse
        let reparsed = TargetTriple::parse(host.as_str());
        assert!(
            reparsed.is_ok(),
            "host() returned unrecognised triple: {}",
            host.as_str()
        );
    }

    #[test]
    fn clang_target_flags_include_target_arg() {
        let t = TargetTriple::parse("aarch64-unknown-linux-gnu").unwrap();
        let flags = t.clang_target_flags();
        assert_eq!(flags, vec!["-target", "aarch64-unknown-linux-gnu"]);
    }

    #[test]
    fn cross_compiler_binary_aarch64_linux() {
        let t = TargetTriple::parse("aarch64-unknown-linux-gnu").unwrap();
        // On a Linux x86_64 host this is a cross target
        if !t.is_host() {
            assert_eq!(
                t.cross_compiler_binary(),
                Some("aarch64-linux-gnu-gcc".to_string())
            );
        }
    }

    #[test]
    fn cross_compiler_binary_host_returns_none() {
        let host = TargetTriple::host();
        assert_eq!(host.cross_compiler_binary(), None);
    }

    #[test]
    fn all_targets_have_no_exe_extension() {
        for triple in [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        ] {
            let t = TargetTriple::parse(triple).unwrap();
            assert_eq!(t.exe_extension(), "");
        }
    }

    #[test]
    fn compiler_flags_for_all_cross_targets() {
        // Verify that the correct cross-compiler binary names are generated
        // for each non-host target (tests flag generation, not actual compilation).
        struct Expected {
            triple: &'static str,
            gcc_binary: Option<&'static str>,
        }

        let cases = [
            Expected {
                triple: "x86_64-unknown-linux-gnu",
                gcc_binary: Some("x86_64-linux-gnu-gcc"),
            },
            Expected {
                triple: "aarch64-unknown-linux-gnu",
                gcc_binary: Some("aarch64-linux-gnu-gcc"),
            },
            Expected {
                triple: "x86_64-apple-darwin",
                gcc_binary: None, // clang-only
            },
            Expected {
                triple: "aarch64-apple-darwin",
                gcc_binary: None, // clang-only
            },
        ];

        for case in cases {
            let t = TargetTriple::parse(case.triple).unwrap();
            if t.is_host() {
                // host target always returns None from cross_compiler_binary
                assert_eq!(t.cross_compiler_binary(), None);
            } else {
                assert_eq!(
                    t.cross_compiler_binary(),
                    case.gcc_binary.map(|s| s.to_string()),
                    "wrong gcc binary for {}",
                    case.triple
                );
            }

            // clang flags always include -target <triple>
            assert_eq!(
                t.clang_target_flags(),
                vec!["-target".to_string(), case.triple.to_string()],
                "wrong clang flags for {}",
                case.triple
            );
        }
    }
}
