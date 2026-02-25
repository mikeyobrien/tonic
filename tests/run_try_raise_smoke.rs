use std::fs;
mod common;

#[test]
fn run_try_raise_rescued_smoke() {
    let fixture_root = common::unique_fixture_root("run-try-raise-rescued");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_raise.tn"),
        r#"defmodule Demo do
  def run() do
    try do
      raise("boom")
    rescue
      "boom" -> :ok
    end
  end
end
"#,
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/try_raise.tn"])
        .output()
        .expect("run command should execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "expected successful run of try/raise rescued, got status {:?} and stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        stdout.trim(),
        ":ok",
        "rescued branch should evaluate to :ok"
    );
}

#[test]
fn run_try_raise_unrescued_smoke() {
    let fixture_root = common::unique_fixture_root("run-try-raise-unrescued");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_raise_unrescued.tn"),
        r#"defmodule Demo do
  def run() do
    try do
      raise("boom")
    rescue
      "other" -> :ok
    end
  end
end
"#,
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/try_raise_unrescued.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run of unrescued try/raise to fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("boom"),
        "expected unrescued error message in stderr, got: {}",
        stderr
    );
}

#[test]
fn run_raise_unrescued_deterministic_smoke() {
    let fixture_root = common::unique_fixture_root("run-raise-unrescued");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("raise_unrescued.tn"),
        r#"defmodule Demo do
  def run() do
    raise("deterministic error failure")
  end
end
"#,
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/raise_unrescued.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run of unrescued raise to fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("deterministic error failure"),
        "expected unrescued error message in stderr, got: {}",
        stderr
    );
}

#[test]
fn run_structured_raise_rescue_module_match_and_value_extraction() {
    let fixture_root = common::unique_fixture_root("run-structured-raise-rescue-module");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("structured_raise_rescue.tn"),
        r#"defmodule Demo do
  def run() do
    try do
      raise ValidationError, message: "bad input", field: 42
    rescue
      TimeoutError -> :timeout
      err in ValidationError -> {err.message, err.metadata[:field]}
    end
  end
end
"#,
    )
    .expect("fixture setup should write structured raise source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/structured_raise_rescue.tn"])
        .output()
        .expect("run command should execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "expected successful run of structured raise rescue, got status {:?} and stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        stdout.trim(),
        r#"{"bad input", 42}"#,
        "structured rescue should match by module and expose payload"
    );
}

#[test]
fn run_structured_rescue_module_clause_falls_through_for_string_raise() {
    let fixture_root = common::unique_fixture_root("run-structured-rescue-fallback");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("structured_rescue_fallback.tn"),
        r#"defmodule Demo do
  def run() do
    try do
      raise "boom"
    rescue
      ValidationError -> :typed
      err -> err
    end
  end
end
"#,
    )
    .expect("fixture setup should write fallback source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/structured_rescue_fallback.tn"])
        .output()
        .expect("run command should execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "expected successful run of fallback rescue, got status {:?} and stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        stdout.trim(),
        r#""boom""#,
        "string raise should still be rescued by fallback branch"
    );
}
