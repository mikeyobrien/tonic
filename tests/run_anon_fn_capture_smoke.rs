use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_anonymous_function_with_lexical_capture() {
    let fixture_root = unique_fixture_root("run-anon-fn-lexical-capture");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_anon_fn_capture.tn"),
        "defmodule Demo do\n  def make_adder(base) do\n    fn value -> value + base end\n  end\n\n  def run() do\n    make_adder(4).(3)\n  end\nend\n",
    )
    .expect("fixture setup should write anonymous function source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_anon_fn_capture.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "7\n"
    );
}

#[test]
fn run_executes_capture_shorthand_with_placeholder_expansion() {
    let fixture_root = unique_fixture_root("run-capture-shorthand");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_capture_shorthand.tn"),
        "defmodule Demo do\n  def run() do\n    (&(&1 + 1)).(41)\n  end\nend\n",
    )
    .expect("fixture setup should write capture shorthand source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_capture_shorthand.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "42\n"
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
