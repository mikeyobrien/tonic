use std::fs;
use std::path::PathBuf;

#[test]
fn check_reports_deterministic_undefined_symbol_error_code() {
    let fixture_root = unique_fixture_root("check-undefined-symbol");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_undefined_symbol.tn"),
        "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n",
    )
    .expect("fixture setup should write undefined symbol source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_undefined_symbol.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for undefined symbol, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert_eq!(
        stderr,
        "error: [E1001] undefined symbol 'missing' in Demo.run\n"
    );
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}
