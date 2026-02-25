use std::fs;
mod common;

#[test]
fn check_dump_ast_matches_try_raise_contract() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-try-raise");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_raise_smoke.tn"),
        "defmodule Demo do
  def run() do
    try do
      raise(:boom)
    rescue
      _ -> :ok
    end
  end
end
",
    )
    .expect("fixture setup should write parser smoke source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/try_raise_smoke.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected try/raise to parse successfully, got status {:?} and stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#""kind":"try""#),
        "AST output should contain Try expression"
    );
    assert!(
        stdout.contains(r#""kind":"raise""#),
        "AST output should contain Raise expression"
    );
}

#[test]
fn check_try_catch_after_supported() {
    let fixture_root = common::unique_fixture_root("check-try-catch-after-supported");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_catch_after.tn"),
        "defmodule Demo do
  def run() do
    try do
      :ok
    catch
      _ -> :ok
    after
      :cleanup
    end
  end
end
",
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/try_catch_after.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#""catch""#),
        "AST output should contain catch array"
    );
    assert!(
        stdout.contains(r#""after""#),
        "AST output should contain after block"
    );
}

#[test]
fn check_try_missing_clauses_diagnostic() {
    let fixture_root = common::unique_fixture_root("check-try-missing-clauses");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_missing.tn"),
        "defmodule Demo do
  def run() do
    try do
      :ok
    end
  end
end
",
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/try_missing.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("try must be followed by rescue, catch, or after"),
        "should report missing try clauses"
    );
}

#[test]
fn check_dump_ast_supports_structured_raise_and_rescue_module_clause() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-structured-raise");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("structured_raise.tn"),
        r#"defmodule Demo do
  def run() do
    try do
      raise ValidationError, message: "bad input", field: 42
    rescue
      err in ValidationError -> {err.message, err.metadata[:field]}
    end
  end
end
"#,
    )
    .expect("fixture setup should write structured raise source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/structured_raise.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected structured raise to parse successfully, got status {:?} and stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("__exception__"),
        "structured raise should include exception marker in AST"
    );
    assert!(
        stdout.contains("metadata"),
        "structured raise should include metadata payload in AST"
    );
}

#[test]
fn check_structured_raise_invalid_keyword_args_diagnostic() {
    let fixture_root = common::unique_fixture_root("check-structured-raise-invalid-args");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("structured_raise_invalid.tn"),
        r#"defmodule Demo do
  def run() do
    raise ValidationError, "boom"
  end
end
"#,
    )
    .expect("fixture setup should write invalid structured raise source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/structured_raise_invalid.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("structured raise expects keyword arguments"),
        "should report invalid structured raise arguments"
    );
}

#[test]
fn check_rescue_module_match_requires_module_reference_diagnostic() {
    let fixture_root = common::unique_fixture_root("check-rescue-module-match-invalid");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("rescue_module_invalid.tn"),
        r#"defmodule Demo do
  def run() do
    try do
      raise ValidationError, message: "boom"
    rescue
      err in :validation_error -> err
    end
  end
end
"#,
    )
    .expect("fixture setup should write invalid rescue source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/rescue_module_invalid.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("rescue module match expects module reference"),
        "should report invalid rescue module match"
    );
}
