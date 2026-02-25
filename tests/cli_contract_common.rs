use std::process::Command;

fn get_tonic_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tonic")
}

fn assert_usage_error(output: std::process::Output, expected_stderr: &str) {
    assert!(
        !output.status.success(),
        "Expected usage error, but command succeeded"
    );
    assert_eq!(
        output.status.code(),
        Some(64),
        "Expected usage error exit code 64, got {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("error:"),
        "Expected stderr to contain 'error:', got: {}",
        stderr
    );
    assert!(
        stderr.contains(expected_stderr),
        "Expected stderr to contain '{}', got: {}",
        expected_stderr,
        stderr
    );
}

fn assert_runtime_error(output: std::process::Output, expected_stderr: &str) {
    assert!(
        !output.status.success(),
        "Expected runtime error, but command succeeded"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "Expected runtime error exit code 1, got {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("error:"),
        "Expected stderr to contain 'error:', got: {}",
        stderr
    );
    assert!(
        stderr.contains(expected_stderr),
        "Expected stderr to contain '{}', got: {}",
        expected_stderr,
        stderr
    );
}

// Commands that take a `<path>` as their first argument
const PATH_COMMANDS: &[&str] = &["run", "check", "test", "fmt", "compile"];

#[test]
fn common_path_commands_no_args_is_usage_error() {
    for cmd in PATH_COMMANDS {
        let output = Command::new(get_tonic_bin()).arg(cmd).output().unwrap();
        assert_usage_error(output, "missing required <path>");
    }
}

#[test]
fn common_path_commands_extra_args_is_usage_error() {
    for cmd in PATH_COMMANDS {
        if *cmd == "run" {
            continue; // `run` accepts extra arguments for `System.argv()`
        }

        let mut command = Command::new(get_tonic_bin());
        command.arg(cmd).arg("dummy_path.tn");

        // compile allows --out, format allows --check
        if *cmd == "compile" {
            command.args(["--out", "dummy_out.json", "extra"]);
        } else if *cmd == "fmt" {
            command.args(["--check", "extra"]);
        } else if *cmd == "check" {
            command.args(["--dump-tokens", "extra"]);
        } else {
            command.arg("extra");
        }

        let output = command.output().unwrap();
        assert_usage_error(output, "unexpected argument 'extra'");
    }
}

#[test]
fn common_path_commands_missing_file_is_runtime_error() {
    for cmd in PATH_COMMANDS {
        let output = Command::new(get_tonic_bin())
            .args([cmd, "does_not_exist.tn"])
            .output()
            .unwrap();
        assert_runtime_error(output, "");
    }
}

#[test]
fn verify_run_no_args_is_usage_error() {
    let output = Command::new(get_tonic_bin())
        .args(["verify", "run"])
        .output()
        .unwrap();
    assert_usage_error(output, "missing required <slice-id>");
}

#[test]
fn verify_run_extra_args_is_usage_error() {
    let output = Command::new(get_tonic_bin())
        .args(["verify", "run", "slice-id", "--mode", "auto", "extra"])
        .output()
        .unwrap();
    assert_usage_error(output, "unexpected argument 'extra'");
}

#[test]
fn verify_run_missing_slice_is_runtime_error() {
    let output = Command::new(get_tonic_bin())
        .args(["verify", "run", "does_not_exist_slice"])
        .output()
        .unwrap();
    assert_runtime_error(output, "missing acceptance file");
}
