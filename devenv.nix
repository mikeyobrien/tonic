{ pkgs, ... }:

{
  languages.rust.enable = true;

  packages = with pkgs; [
    git
    pkg-config
    openssl
  ];

  scripts = {
    test.exec = "cargo test";
    check.exec = "cargo check --all-targets --all-features";
    fmt.exec = "cargo fmt --all -- --check";
    lint.exec = "cargo clippy --all-targets --all-features -- -D warnings";
  };

  enterShell = ''
    echo "devenv ready for tonic"
    rustc --version
    cargo --version
  '';
}
