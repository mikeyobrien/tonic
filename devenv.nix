{ pkgs, ... }:

{
  languages.rust.enable = true;

  packages = with pkgs; [
    git
    pkg-config
    openssl
  ];

  scripts = {
    test.exec = ''
      if [ -n "''${TONIC_OBS_ENABLE:-}" ]; then
        source ./scripts/lib/observability.sh
        tonic_obs_run_step "dev-test" cargo test --all-features
      else
        cargo test --all-features
      fi
    '';
    check.exec = ''
      if [ -n "''${TONIC_OBS_ENABLE:-}" ]; then
        source ./scripts/lib/observability.sh
        tonic_obs_run_step "dev-check" cargo check --all-targets --all-features
      else
        cargo check --all-targets --all-features
      fi
    '';
    fmt.exec = "cargo fmt --all -- --check";
    lint.exec = ''
      if [ -n "''${TONIC_OBS_ENABLE:-}" ]; then
        source ./scripts/lib/observability.sh
        tonic_obs_run_step "dev-lint" cargo clippy --all-targets --all-features -- -D warnings
      else
        cargo clippy --all-targets --all-features -- -D warnings
      fi
    '';
  };

  enterShell = ''
    echo "devenv ready for tonic"
    rustc --version
    cargo --version
  '';
}
