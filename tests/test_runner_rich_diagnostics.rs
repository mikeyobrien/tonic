use serde_json::Value;
use std::fs;
use std::path::PathBuf;
mod common;

#[test]
fn test_project_root_discovers_and_runs_test_files() {
    let fixture_root = common::unique_fixture_root("test-runner-project-root");
    let src_dir = fixture_root.join("src");
    let tests_dir = fixture_root.join("tests");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&tests_dir).expect("fixture setup should create tests directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write main module");
    fs::write(
        tests_dir.join("math_test.tn"),
        "defmodule MathTest do\n  def test_add() do\n    1 + 1\n  end\nend\n",
    )
    .expect("fixture setup should write test module");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "expected test command success, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("test MathTest.test_add ... ok ("));
    assert!(stdout.contains("test result: ok. 1 passed; 0 failed; 1 total; finished in "));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

#[test]
fn test_returns_non_zero_and_deterministic_summary_when_failures_exist() {
    let fixture_root = write_single_test_file(
        "test-runner-failure-summary",
        "failing_test.tn",
        "defmodule FailingTest do\n  def test_ok() do\n    42\n  end\n\n  def test_fail() do\n    err(7)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when at least one test fails"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("test FailingTest.test_fail ... FAILED ("));
    assert!(stdout.contains("error: runtime returned err(7)"));
    assert!(stdout.contains("test FailingTest.test_ok ... ok ("));
    assert!(stdout.contains("test result: FAILED. 1 passed; 1 failed; 2 total; finished in "));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

#[test]
fn test_file_target_mode_executes_tests_from_explicit_file_path() {
    let fixture_root = write_single_test_file(
        "test-runner-file-target",
        "manual_suite.tn",
        "defmodule ManualSuite do\n  def test_manual_case() do\n    :ok\n  end\nend\n",
    );

    let file_path = fixture_root.join("manual_suite.tn");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", file_path.to_str().expect("utf8 file path")])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "expected explicit file path to pass"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("test ManualSuite.test_manual_case ... ok ("));
    assert!(stdout.contains("test result: ok. 1 passed; 0 failed; 1 total; finished in "));
}

#[test]
fn test_supports_machine_readable_json_output() {
    let fixture_root = write_single_test_file(
        "test-runner-json-output",
        "json_mode_test.tn",
        "defmodule JsonModeTest do\n  def test_alpha() do\n    1\n  end\n\n  def test_beta() do\n    err(:boom)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--format", "json"])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json output should parse");

    assert_eq!(report["status"], "failed");
    assert_eq!(report["total"], 2);
    assert_eq!(report["passed"], 1);
    assert_eq!(report["failed"], 1);

    let results = report["results"]
        .as_array()
        .expect("results should be array");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["id"], "JsonModeTest.test_alpha");
    assert_eq!(results[0]["status"], "passed");
    assert_eq!(results[1]["id"], "JsonModeTest.test_beta");
    assert_eq!(results[1]["status"], "failed");
    assert_eq!(results[1]["error"], "runtime returned err(:boom)");

    // Per-test and total timing fields should be present
    assert!(
        report["duration_ms"].is_number(),
        "report should include duration_ms, got: {report}"
    );
    assert!(
        results[0]["duration_ms"].is_number(),
        "each result should include duration_ms, got: {:?}",
        results[0]
    );
}

#[test]
fn check_diagnostics_include_line_column_and_source_snippet() {
    let fixture_root = common::unique_fixture_root("check-rich-resolver-diagnostics");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_error.tn"),
        "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n",
    )
    .expect("fixture setup should write resolver fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_error.tn"])
        .output()
        .expect("check command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("error: [E1001] undefined symbol 'missing' in Demo.run"));
    assert!(
        stderr.contains("--> examples/resolver_error.tn:3:5"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     missing()"));
    assert!(stderr.contains("|     ^"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_frontend_errors() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-front-end",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    %{1 2}\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0008] missing '=>' in map entry; found INT(2) instead."));
    assert!(stderr.contains("hint: write `%{key => value}` for computed keys"));
    assert!(
        stderr.contains("invalid_test.tn:3:"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     %{1 2}"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_missing_call_commas() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-missing-call-comma",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    tuple 1 2\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0010] missing ',' in call arguments; found INT(2) instead."));
    assert!(stderr.contains("hint: separate call arguments with commas"));
    assert!(
        stderr.contains("invalid_test.tn:3:"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     tuple 1 2"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_unclosed_call_delimiters() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-unclosed-call-delimiter",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    Math.add(1, 2\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0002] unclosed delimiter: call argument list is missing ')'."));
    assert!(stderr.contains("hint: add ')' to close the call arguments"));
    assert!(
        stderr.contains("invalid_test.tn:3:13"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     Math.add(1, 2"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_missing_with_clause_commas() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-missing-with-clause-comma",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    with ok <- ok(1)\n         value <- ok + 1 do\n      value\n    end\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0010] missing ',' in with clauses; found IDENT(value) instead."));
    assert!(stderr.contains("hint: separate with clauses with commas"));
    assert!(
        stderr.contains("invalid_test.tn:4:"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("4 |          value <- ok + 1 do"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_missing_alias_child_commas() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-missing-alias-child-comma",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  alias Math.{Add Sub}\n\n  def test_bad() do\n    Add.value()\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0010] missing ',' in alias child list; found IDENT(Sub) instead."));
    assert!(stderr.contains("hint: separate alias children with commas"));
    assert!(
        stderr.contains("invalid_test.tn:2:"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("2 |   alias Math.{Add Sub}"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_unclosed_structured_raise_arguments() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-unclosed-structured-raise-arguments",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    raise(RuntimeError, message: \"oops\"\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("[E0002] unclosed delimiter: structured raise arguments is missing ')'.")
    );
    assert!(stderr.contains("hint: add ')' to close the structured raise arguments"));
    assert!(
        stderr.contains("invalid_test.tn:3:10"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     raise(RuntimeError, message: \"oops\""));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_missing_keyword_list_commas() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-missing-keyword-list-comma",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    [message: \"oops\" detail: 1]\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0010] missing ',' in keyword list; found IDENT(detail) instead."));
    assert!(stderr.contains("hint: separate keyword entries with commas"));
    assert!(
        stderr.contains("invalid_test.tn:3:22"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     [message: \"oops\" detail: 1]"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_unclosed_list_patterns() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-unclosed-list-pattern",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad(value) do\n    case value do\n      [head, tail -> head\n    end\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0002] unclosed delimiter: list pattern is missing ']'."));
    assert!(stderr.contains("hint: add ']' to close the list pattern"));
    assert!(
        stderr.contains("invalid_test.tn:4:7"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("4 |       [head, tail -> head"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_missing_bitstring_commas() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-missing-bitstring-comma",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    <<1 2>>\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0010] missing ',' in bitstring literal; found INT(2) instead."));
    assert!(stderr.contains("hint: separate bitstring elements with commas"));
    assert!(
        stderr.contains("invalid_test.tn:3:9"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     <<1 2>>"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_unclosed_bitstring_patterns() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-unclosed-bitstring-pattern",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad(value) do\n    case value do\n      <<left, right -> left\n    end\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E0002] unclosed delimiter: bitstring pattern is missing '>>'."));
    assert!(stderr.contains("hint: add '>>' to close the bitstring pattern"));
    assert!(
        stderr.contains("invalid_test.tn:4:7"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("4 |       <<left, right -> left"));
}

// --- Assert module integration tests ---

#[test]
fn test_assert_equal_renders_expected_vs_actual_on_failure() {
    let fixture_root = write_single_test_file(
        "test-assert-equal-failure",
        "test_assert.tn",
        "defmodule AssertTest do\n  def test_equal_pass() do\n    Assert.assert_equal(1, 1)\n  end\n\n  def test_equal_fail() do\n    Assert.assert_equal(1, 2)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test AssertTest.test_equal_pass ... ok ("),
        "passing assert_equal should show ok, got: {stdout}"
    );
    assert!(
        stdout.contains("test AssertTest.test_equal_fail ... FAILED ("),
        "failing assert_equal should show FAILED, got: {stdout}"
    );
    assert!(
        stdout.contains("left:  1"),
        "failure output should show left value, got: {stdout}"
    );
    assert!(
        stdout.contains("right: 2"),
        "failure output should show right value, got: {stdout}"
    );
}

#[test]
fn test_assert_renders_truthy_failure() {
    let fixture_root = write_single_test_file(
        "test-assert-truthy-failure",
        "test_assert_truthy.tn",
        "defmodule AssertTruthyTest do\n  def test_assert_true() do\n    Assert.assert(true)\n  end\n\n  def test_assert_false() do\n    Assert.assert(false)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test AssertTruthyTest.test_assert_true ... ok ("),
        "assert(true) should pass, got: {stdout}"
    );
    assert!(
        stdout.contains("test AssertTruthyTest.test_assert_false ... FAILED ("),
        "assert(false) should fail, got: {stdout}"
    );
    assert!(
        stdout.contains("assert failed:"),
        "failure output should say 'assert failed:', got: {stdout}"
    );
}

#[test]
fn test_refute_renders_falsy_failure() {
    let fixture_root = write_single_test_file(
        "test-refute-falsy-failure",
        "test_refute.tn",
        "defmodule RefuteTest do\n  def test_refute_false() do\n    Assert.refute(false)\n  end\n\n  def test_refute_true() do\n    Assert.refute(true)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test RefuteTest.test_refute_false ... ok ("),
        "refute(false) should pass, got: {stdout}"
    );
    assert!(
        stdout.contains("test RefuteTest.test_refute_true ... FAILED ("),
        "refute(true) should fail, got: {stdout}"
    );
    assert!(
        stdout.contains("refute failed:"),
        "failure output should say 'refute failed:', got: {stdout}"
    );
}

#[test]
fn test_assert_not_equal_renders_failure() {
    let fixture_root = write_single_test_file(
        "test-assert-not-equal-failure",
        "test_assert_neq.tn",
        "defmodule AssertNeqTest do\n  def test_not_equal_pass() do\n    Assert.assert_not_equal(1, 2)\n  end\n\n  def test_not_equal_fail() do\n    Assert.assert_not_equal(1, 1)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test AssertNeqTest.test_not_equal_pass ... ok ("),
        "assert_not_equal(1, 2) should pass, got: {stdout}"
    );
    assert!(
        stdout.contains("test AssertNeqTest.test_not_equal_fail ... FAILED ("),
        "assert_not_equal(1, 1) should fail, got: {stdout}"
    );
    assert!(
        stdout.contains("assert_not_equal failed:"),
        "failure output should say 'assert_not_equal failed:', got: {stdout}"
    );
}

#[test]
fn test_assert_equal_with_custom_message() {
    let fixture_root = write_single_test_file(
        "test-assert-equal-custom-msg",
        "test_assert_msg.tn",
        "defmodule AssertMsgTest do\n  def test_custom_message() do\n    Assert.assert_equal(1, 2, \"expected same value\")\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("expected same value"),
        "failure output should include custom message, got: {stdout}"
    );
}

#[test]
fn test_assert_json_output_includes_structured_error() {
    let fixture_root = write_single_test_file(
        "test-assert-json-output",
        "test_assert_json.tn",
        "defmodule AssertJsonTest do\n  def test_fail() do\n    Assert.assert_equal(\"a\", \"b\")\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .args([
            "test",
            &fixture_root.display().to_string(),
            "--format",
            "json",
        ])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let json: Value = serde_json::from_str(stdout.trim()).expect("output should be valid JSON");
    let results = json["results"].as_array().expect("results should be array");
    assert_eq!(results.len(), 1);
    let error = results[0]["error"]
        .as_str()
        .expect("error should be string");
    assert!(
        error.contains("left:"),
        "JSON error should include left value, got: {error}"
    );
    assert!(
        error.contains("right:"),
        "JSON error should include right value, got: {error}"
    );
}

// --- --filter integration tests ---

#[test]
fn test_filter_runs_only_matching_tests() {
    let fixture_root = write_single_test_file(
        "test-filter-subset",
        "test_filter.tn",
        "defmodule FilterTest do\n  def test_alpha() do\n    1\n  end\n\n  def test_beta() do\n    2\n  end\n\n  def test_gamma() do\n    3\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--filter", "beta"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "expected test command success, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test FilterTest.test_beta ... ok ("),
        "filtered test should appear, got: {stdout}"
    );
    assert!(
        !stdout.contains("test_alpha"),
        "non-matching test should be excluded, got: {stdout}"
    );
    assert!(
        !stdout.contains("test_gamma"),
        "non-matching test should be excluded, got: {stdout}"
    );
    assert!(
        stdout.contains("1 passed; 0 failed; 1 total"),
        "summary should reflect filtered count, got: {stdout}"
    );
}

#[test]
fn test_filter_no_matches_reports_zero_tests() {
    let fixture_root = write_single_test_file(
        "test-filter-none",
        "test_filter_none.tn",
        "defmodule FilterNoneTest do\n  def test_alpha() do\n    1\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--filter", "nonexistent"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "zero tests matching filter should still succeed"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("0 passed; 0 failed; 0 total"),
        "summary should show zero tests, got: {stdout}"
    );
}

#[test]
fn test_filter_with_json_output() {
    let fixture_root = write_single_test_file(
        "test-filter-json",
        "test_filter_json.tn",
        "defmodule FilterJsonTest do\n  def test_one() do\n    1\n  end\n\n  def test_two() do\n    2\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--filter", "one", "--format", "json"])
        .output()
        .expect("test command should execute");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json output should parse");

    assert_eq!(report["total"], 1);
    assert_eq!(report["passed"], 1);

    let results = report["results"]
        .as_array()
        .expect("results should be array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["id"], "FilterJsonTest.test_one");
    assert!(
        results[0]["duration_ms"].is_number(),
        "filtered result should include duration_ms"
    );
    assert!(
        report["duration_ms"].is_number(),
        "filtered report should include duration_ms"
    );
}

// --- Failure summary section tests ---

#[test]
fn test_failure_summary_section_appears_for_mixed_pass_fail() {
    let fixture_root = write_single_test_file(
        "test-failure-summary-mixed",
        "test_summary.tn",
        "defmodule SummaryTest do\n  def test_good() do\n    1\n  end\n\n  def test_bad() do\n    err(:oops)\n  end\n\n  def test_also_bad() do\n    err(:boom)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    // Failure summary section should be present
    assert!(
        stdout.contains("Failures:"),
        "expected 'Failures:' section header, got: {stdout}"
    );
    assert!(
        stdout.contains("1. SummaryTest.test_also_bad"),
        "expected numbered failure entry for test_also_bad, got: {stdout}"
    );
    assert!(
        stdout.contains("2. SummaryTest.test_bad"),
        "expected numbered failure entry for test_bad, got: {stdout}"
    );
    assert!(
        stdout.contains("test result: FAILED. 1 passed; 2 failed; 3 total"),
        "expected summary line, got: {stdout}"
    );
}

#[test]
fn test_failure_summary_section_absent_when_all_pass() {
    let fixture_root = write_single_test_file(
        "test-failure-summary-all-pass",
        "test_all_pass.tn",
        "defmodule AllPassTest do\n  def test_one() do\n    1\n  end\n\n  def test_two() do\n    2\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    assert!(
        !stdout.contains("Failures:"),
        "no 'Failures:' section when all tests pass, got: {stdout}"
    );
}

#[test]
fn test_json_output_includes_failures_array() {
    let fixture_root = write_single_test_file(
        "test-failure-summary-json",
        "test_json_failures.tn",
        "defmodule JsonFailuresTest do\n  def test_pass() do\n    1\n  end\n\n  def test_fail() do\n    err(:nope)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--format", "json"])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json output should parse");

    let failures = report["failures"]
        .as_array()
        .expect("failures should be array");
    assert_eq!(failures.len(), 1, "only one test failed");
    assert_eq!(failures[0]["id"], "JsonFailuresTest.test_fail");
    assert_eq!(failures[0]["status"], "failed");
    assert!(
        failures[0]["error"].is_string(),
        "failure entry should include error"
    );

    // All-pass JSON should have empty failures array
    let all_results = report["results"]
        .as_array()
        .expect("results should be array");
    assert_eq!(all_results.len(), 2, "all results should still be present");
}

// --- --list flag tests ---

#[test]
fn test_list_flag_shows_test_names_without_running() {
    let fixture_root = write_single_test_file(
        "test-list-text",
        "test_list.tn",
        "defmodule ListTest do\n  def test_alpha() do\n    1\n  end\n\n  def test_beta() do\n    err(:boom)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--list"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "list should succeed even when tests would fail, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("ListTest.test_alpha"),
        "expected test_alpha in list, got: {stdout}"
    );
    assert!(
        stdout.contains("ListTest.test_beta"),
        "expected test_beta in list, got: {stdout}"
    );
    // Should NOT contain test runner output markers
    assert!(
        !stdout.contains("test result:"),
        "list should not run tests, got: {stdout}"
    );
}

#[test]
fn test_list_flag_json_output() {
    let fixture_root = write_single_test_file(
        "test-list-json",
        "test_list_json.tn",
        "defmodule ListJsonTest do\n  def test_one() do\n    1\n  end\n\n  def test_two() do\n    2\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--list", "--format", "json"])
        .output()
        .expect("test command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json output should parse");

    let tests = report["tests"].as_array().expect("tests should be array");
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0], "ListJsonTest.test_one");
    assert_eq!(tests[1], "ListJsonTest.test_two");
}

#[test]
fn test_list_flag_with_filter() {
    let fixture_root = write_single_test_file(
        "test-list-filter",
        "test_list_filter.tn",
        "defmodule ListFilterTest do\n  def test_alpha() do\n    1\n  end\n\n  def test_beta() do\n    2\n  end\n\n  def test_gamma() do\n    3\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", ".", "--list", "--filter", "alpha"])
        .output()
        .expect("test command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines.len(), 1, "only alpha should match, got: {stdout}");
    assert_eq!(lines[0], "ListFilterTest.test_alpha");
}

fn write_single_test_file(test_name: &str, file_name: &str, source: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);

    fs::write(fixture_root.join(file_name), source).expect("fixture setup should write test file");

    fixture_root
}

// ── assert_contains tests ──────────────────────────────────────────────────

#[test]
fn test_assert_contains_string_pass_and_fail() {
    let fixture_root = write_single_test_file(
        "test-assert-contains-string",
        "contains_test.tn",
        "defmodule ContainsTest do
  def test_string_contains_pass() do
    Assert.assert_contains(\"hello world\", \"world\")
  end

  def test_string_contains_fail() do
    Assert.assert_contains(\"hello world\", \"xyz\")
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "contains_test.tn"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test ContainsTest.test_string_contains_pass ... ok"),
        "expected pass for string contains, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test ContainsTest.test_string_contains_fail ... FAIL"),
        "expected fail for missing substring, got:\n{stdout}"
    );
    assert!(
        stdout.contains("assert_contains failed"),
        "expected structured assert_contains failure, got:\n{stdout}"
    );
    assert!(
        stdout.contains("container:"),
        "expected container field in failure output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("element:"),
        "expected element field in failure output, got:\n{stdout}"
    );
}

#[test]
fn test_assert_contains_list_pass_and_fail() {
    let fixture_root = write_single_test_file(
        "test-assert-contains-list",
        "list_contains_test.tn",
        "defmodule ListContainsTest do
  def test_list_contains_pass() do
    Assert.assert_contains([1, 2, 3], 2)
  end

  def test_list_contains_fail() do
    Assert.assert_contains([1, 2, 3], 99)
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "list_contains_test.tn"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test ListContainsTest.test_list_contains_pass ... ok"),
        "expected pass for list contains, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test ListContainsTest.test_list_contains_fail ... FAIL"),
        "expected fail for missing element, got:\n{stdout}"
    );
}

// ── assert_in_delta tests ──────────────────────────────────────────────────

#[test]
fn test_assert_in_delta_pass_and_fail() {
    let fixture_root = write_single_test_file(
        "test-assert-in-delta",
        "delta_test.tn",
        "defmodule DeltaTest do
  def test_in_delta_pass() do
    Assert.assert_in_delta(10, 11, 2)
  end

  def test_in_delta_fail() do
    Assert.assert_in_delta(10, 20, 2)
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "delta_test.tn"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test DeltaTest.test_in_delta_pass ... ok"),
        "expected pass for values within delta, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test DeltaTest.test_in_delta_fail ... FAIL"),
        "expected fail for values outside delta, got:\n{stdout}"
    );
    assert!(
        stdout.contains("assert_in_delta failed"),
        "expected structured assert_in_delta failure, got:\n{stdout}"
    );
    assert!(
        stdout.contains("delta:"),
        "expected delta field in failure output, got:\n{stdout}"
    );
}

#[test]
fn test_assert_in_delta_json_output() {
    let fixture_root = write_single_test_file(
        "test-assert-in-delta-json",
        "delta_json_test.tn",
        "defmodule DeltaJsonTest do
  def test_in_delta_fail() do
    Assert.assert_in_delta(1, 100, 5, \"too far apart\")
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "delta_json_test.tn", "--format", "json"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let json: Value = serde_json::from_str(&stdout).expect("should parse JSON output");
    let results = json["results"].as_array().expect("results should be array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["status"], "failed");
    let error = results[0]["error"]
        .as_str()
        .expect("error should be string");
    assert!(
        error.contains("assert_in_delta failed"),
        "expected assert_in_delta failure in JSON error, got: {error}"
    );
}

// ── --fail-fast tests ──────────────────────────────────────────────────────

#[test]
fn test_fail_fast_stops_after_first_failure() {
    let fixture_root = write_single_test_file(
        "test-fail-fast-stops",
        "fail_fast_test.tn",
        "defmodule FailFastTest do
  def test_alpha() do
    :ok
  end

  def test_beta_fail() do
    err(:boom)
  end

  def test_gamma() do
    :ok
  end

  def test_delta_fail() do
    err(:bang)
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "fail_fast_test.tn", "--fail-fast"])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1), "should fail");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    // Tests are sorted alphabetically: alpha, beta_fail, delta_fail, gamma
    // With --fail-fast, only alpha (pass) and beta_fail (fail) should appear
    assert!(
        stdout.contains("test FailFastTest.test_alpha ... ok"),
        "alpha should have run, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test FailFastTest.test_beta_fail ... FAILED"),
        "beta_fail should have run, got:\n{stdout}"
    );
    // delta_fail and gamma should NOT appear because we stopped at beta_fail
    assert!(
        !stdout.contains("test_delta_fail"),
        "delta_fail should not have run with --fail-fast, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("test_gamma"),
        "gamma should not have run with --fail-fast, got:\n{stdout}"
    );
    assert!(
        stdout.contains("1 passed; 1 failed; 2 total"),
        "summary should reflect only executed tests, got:\n{stdout}"
    );
}

#[test]
fn test_fail_fast_runs_all_when_all_pass() {
    let fixture_root = write_single_test_file(
        "test-fail-fast-all-pass",
        "all_pass_test.tn",
        "defmodule AllPassTest do
  def test_one() do
    :ok
  end

  def test_two() do
    :ok
  end

  def test_three() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "all_pass_test.tn", "--fail-fast"])
        .output()
        .expect("test command should execute");

    assert!(output.status.success(), "all tests should pass");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("3 passed; 0 failed; 3 total"),
        "all three should run when none fail, got:\n{stdout}"
    );
}

#[test]
fn test_fail_fast_json_output() {
    let fixture_root = write_single_test_file(
        "test-fail-fast-json",
        "fail_fast_json_test.tn",
        "defmodule FailFastJsonTest do
  def test_alpha() do
    :ok
  end

  def test_beta_fail() do
    err(:boom)
  end

  def test_gamma() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args([
            "test",
            "fail_fast_json_test.tn",
            "--fail-fast",
            "--format",
            "json",
        ])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let json: Value = serde_json::from_str(&stdout).expect("should parse JSON output");
    let results = json["results"].as_array().expect("results should be array");

    // Only alpha (pass) and beta_fail (fail) should be in results
    assert_eq!(
        results.len(),
        2,
        "should have 2 results (stopped at first failure), got: {results:?}"
    );
    assert_eq!(json["passed"], 1);
    assert_eq!(json["failed"], 1);
    assert_eq!(json["total"], 2);
}

// ── ANSI color tests ──────────────────────────────────────────────────────

#[test]
fn test_text_output_includes_ansi_colors() {
    let fixture_root = write_single_test_file(
        "test-ansi-colors",
        "color_test.tn",
        "defmodule ColorTest do
  def test_pass() do
    :ok
  end

  def test_fail() do
    err(:boom)
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        // Deliberately NOT setting NO_COLOR so ANSI codes appear
        .args(["test", "color_test.tn"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    // Green for passing tests
    assert!(
        stdout.contains("\x1b[32mok\x1b[0m"),
        "passing test should have green ANSI code, got:\n{stdout}"
    );
    // Red for failing tests
    assert!(
        stdout.contains("\x1b[31mFAILED\x1b[0m"),
        "failing test should have red ANSI code, got:\n{stdout}"
    );
    // Bold+red for Failures: header
    assert!(
        stdout.contains("\x1b[1m\x1b[31mFailures:\x1b[0m"),
        "Failures header should have bold+red ANSI code, got:\n{stdout}"
    );
    // Summary line should have red (since there are failures)
    assert!(
        stdout.contains("\x1b[31mFAILED\x1b[0m. "),
        "summary should have red status, got:\n{stdout}"
    );
}

#[test]
fn test_no_color_env_strips_ansi_codes() {
    let fixture_root = write_single_test_file(
        "test-no-color",
        "nocolor_test.tn",
        "defmodule NoColorTest do
  def test_pass() do
    :ok
  end

  def test_fail() do
    err(:boom)
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "nocolor_test.tn"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        !stdout.contains("\x1b["),
        "NO_COLOR should strip all ANSI escape codes, got:\n{stdout}"
    );
    // Verify content is still there
    assert!(stdout.contains("test NoColorTest.test_pass ... ok ("));
    assert!(stdout.contains("test NoColorTest.test_fail ... FAILED ("));
    assert!(stdout.contains("Failures:"));
}

#[test]
fn test_json_output_never_includes_ansi_codes() {
    let fixture_root = write_single_test_file(
        "test-json-no-ansi",
        "json_ansi_test.tn",
        "defmodule JsonAnsiTest do
  def test_pass() do
    :ok
  end

  def test_fail() do
    err(:boom)
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        // Deliberately NOT setting NO_COLOR — JSON should still be clean
        .args(["test", "json_ansi_test.tn", "--format", "json"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        !stdout.contains("\x1b["),
        "JSON output should never contain ANSI codes, got:\n{stdout}"
    );
    let json: Value = serde_json::from_str(&stdout).expect("should parse valid JSON");
    assert_eq!(json["status"], "failed");
}

#[test]
fn test_seed_randomizes_test_order() {
    let fixture_root = common::unique_fixture_root("test-runner-seed-randomizes");

    fs::create_dir_all(&fixture_root).expect("fixture setup");
    // Create enough tests that shuffling with a specific seed will produce a different order
    fs::write(
        fixture_root.join("test_order.tn"),
        "defmodule OrderTest do
  def test_alpha() do :ok end
  def test_bravo() do :ok end
  def test_charlie() do :ok end
  def test_delta() do :ok end
  def test_echo() do :ok end
  def test_foxtrot() do :ok end
end\n",
    )
    .expect("fixture setup");

    // Run without seed (default sorted order)
    let default_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "test_order.tn"])
        .output()
        .expect("test command should execute");
    let default_stdout = String::from_utf8(default_output.stdout).expect("utf8");

    // Run with seed
    let seeded_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "test_order.tn", "--seed", "42"])
        .output()
        .expect("test command should execute");
    let seeded_stdout = String::from_utf8(seeded_output.stdout).expect("utf8");

    assert!(seeded_output.status.success());
    assert!(seeded_stdout.contains("Randomized with seed 42"));
    // The seeded order should differ from default sorted order
    assert_ne!(
        default_stdout, seeded_stdout,
        "seeded output should differ from default sorted output"
    );
}

#[test]
fn test_seed_is_deterministic() {
    let fixture_root = common::unique_fixture_root("test-runner-seed-deterministic");

    fs::create_dir_all(&fixture_root).expect("fixture setup");
    fs::write(
        fixture_root.join("test_det.tn"),
        "defmodule DetTest do
  def test_alpha() do :ok end
  def test_bravo() do :ok end
  def test_charlie() do :ok end
  def test_delta() do :ok end
end\n",
    )
    .expect("fixture setup");

    let run1 = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "test_det.tn", "--seed", "12345"])
        .output()
        .expect("test command should execute");

    let run2 = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "test_det.tn", "--seed", "12345"])
        .output()
        .expect("test command should execute");

    let stdout1 = String::from_utf8(run1.stdout).expect("utf8");
    let stdout2 = String::from_utf8(run2.stdout).expect("utf8");

    // Strip timing info to compare order only (filter lines with " ... " to exclude "test result:")
    let order1: Vec<&str> = stdout1
        .lines()
        .filter(|l| l.starts_with("test ") && l.contains(" ... "))
        .collect();
    let order2: Vec<&str> = stdout2
        .lines()
        .filter(|l| l.starts_with("test ") && l.contains(" ... "))
        .collect();

    assert_eq!(order1.len(), 4);
    // Compare just the test names (strip timing which varies)
    let names1: Vec<&str> = order1
        .iter()
        .map(|l| l.split(" ... ").next().unwrap())
        .collect();
    let names2: Vec<&str> = order2
        .iter()
        .map(|l| l.split(" ... ").next().unwrap())
        .collect();
    assert_eq!(names1, names2, "same seed should produce same test order");
}

#[test]
fn test_seed_json_output() {
    let fixture_root = common::unique_fixture_root("test-runner-seed-json");

    fs::create_dir_all(&fixture_root).expect("fixture setup");
    fs::write(
        fixture_root.join("test_seed_json.tn"),
        "defmodule SeedJsonTest do
  def test_one() do :ok end
  def test_two() do :ok end
end\n",
    )
    .expect("fixture setup");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args([
            "test",
            "test_seed_json.tn",
            "--seed",
            "99",
            "--format",
            "json",
        ])
        .output()
        .expect("test command should execute");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let json: Value = serde_json::from_str(&stdout).expect("should parse valid JSON");
    assert_eq!(json["seed"], 99);
    assert_eq!(json["status"], "ok");
}

// ── setup/0 function tests ─────────────────────────────────────────────────

#[test]
fn test_setup_function_runs_before_each_test() {
    // setup/0 returns ok — both tests should pass since setup doesn't fail.
    // We verify setup ran by having it be a no-op that succeeds, and confirm
    // tests still pass normally.
    let fixture_root = write_single_test_file(
        "test-setup-runs",
        "setup_test.tn",
        "defmodule SetupTest do
  def setup() do
    :ok
  end

  def test_alpha() do
    :ok
  end

  def test_beta() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "setup_test.tn"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "all tests should pass with ok setup"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test SetupTest.test_alpha ... ok"),
        "alpha should pass, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test SetupTest.test_beta ... ok"),
        "beta should pass, got:\n{stdout}"
    );
    assert!(
        stdout.contains("2 passed; 0 failed; 2 total"),
        "summary should show 2 passed, got:\n{stdout}"
    );
    // setup should NOT appear as a test itself
    assert!(
        !stdout.contains("test SetupTest.setup"),
        "setup should not be listed as a test, got:\n{stdout}"
    );
}

#[test]
fn test_setup_failure_marks_test_as_failed() {
    // setup/0 returns err(...) — all tests in that module should fail with "setup failed"
    let fixture_root = write_single_test_file(
        "test-setup-failure",
        "setup_fail_test.tn",
        "defmodule SetupFailTest do
  def setup() do
    err(:setup_broken)
  end

  def test_alpha() do
    :ok
  end

  def test_beta() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "setup_fail_test.tn"])
        .output()
        .expect("test command should execute");

    assert_eq!(
        output.status.code(),
        Some(1),
        "should fail when setup returns err"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test SetupFailTest.test_alpha ... FAILED"),
        "alpha should fail due to setup, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test SetupFailTest.test_beta ... FAILED"),
        "beta should fail due to setup, got:\n{stdout}"
    );
    assert!(
        stdout.contains("setup failed"),
        "error should mention setup failed, got:\n{stdout}"
    );
    assert!(
        stdout.contains("0 passed; 2 failed; 2 total"),
        "summary should show 2 failed, got:\n{stdout}"
    );
}

#[test]
fn test_no_setup_function_works_normally() {
    // Module without setup/0 — tests should run as before (regression guard)
    let fixture_root = write_single_test_file(
        "test-no-setup",
        "no_setup_test.tn",
        "defmodule NoSetupTest do
  def test_pass() do
    :ok
  end

  def test_fail() do
    err(:boom)
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "no_setup_test.tn"])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test NoSetupTest.test_pass ... ok"),
        "pass should work without setup, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test NoSetupTest.test_fail ... FAILED"),
        "fail should work without setup, got:\n{stdout}"
    );
    assert!(
        stdout.contains("1 passed; 1 failed; 2 total"),
        "summary should be normal, got:\n{stdout}"
    );
}

// ── skip tests ─────────────────────────────────────────────────────────────

#[test]
fn test_skip_marks_test_as_skipped() {
    let fixture_root = write_single_test_file(
        "test-skip-basic",
        "skip_test.tn",
        "defmodule SkipTest do
  def test_passes() do
    :ok
  end

  def test_skipped() do
    Assert.skip()
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "skip_test.tn"])
        .output()
        .expect("test command should execute");

    // Skipped tests should not cause a non-zero exit code
    assert!(
        output.status.success(),
        "skip + pass should exit 0, got status {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test SkipTest.test_passes ... ok"),
        "passing test should show ok, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test SkipTest.test_skipped ... skipped"),
        "skipped test should show skipped, got:\n{stdout}"
    );
    assert!(
        stdout.contains("1 passed; 0 failed; 1 skipped; 2 total"),
        "summary should include skipped count, got:\n{stdout}"
    );
}

#[test]
fn test_skip_with_reason() {
    let fixture_root = write_single_test_file(
        "test-skip-reason",
        "skip_reason_test.tn",
        "defmodule SkipReasonTest do
  def test_not_implemented() do
    Assert.skip(\"not implemented yet\")
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "skip_reason_test.tn"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "skipped-only run should exit 0, got status {:?}",
        output.status.code()
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout
            .contains("test SkipReasonTest.test_not_implemented ... skipped (not implemented yet)"),
        "skip reason should appear in output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("0 passed; 0 failed; 1 skipped; 1 total"),
        "summary should count only skipped, got:\n{stdout}"
    );
}

#[test]
fn test_skip_does_not_trigger_fail_fast() {
    let fixture_root = write_single_test_file(
        "test-skip-fail-fast",
        "skip_ff_test.tn",
        "defmodule SkipFfTest do
  def test_alpha_skipped() do
    Assert.skip(\"wip\")
  end

  def test_beta_passes() do
    :ok
  end

  def test_gamma_passes() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "skip_ff_test.tn", "--fail-fast"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "skip + pass with --fail-fast should exit 0, got status {:?}",
        output.status.code()
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    // All 3 tests should appear — skip should not stop --fail-fast execution
    assert!(
        stdout.contains("test SkipFfTest.test_alpha_skipped ... skipped"),
        "skipped test should appear, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test SkipFfTest.test_beta_passes ... ok"),
        "beta should run, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test SkipFfTest.test_gamma_passes ... ok"),
        "gamma should run (skip doesn't trigger fail-fast), got:\n{stdout}"
    );
    assert!(
        stdout.contains("2 passed; 0 failed; 1 skipped; 3 total"),
        "summary should show all 3 tests, got:\n{stdout}"
    );
}

// ── assert_raises tests ────────────────────────────────────────────────────

#[test]
fn test_assert_raises_passes_when_function_raises() {
    let fixture_root = write_single_test_file(
        "test-assert-raises-pass",
        "raises_test.tn",
        "defmodule RaisesTest do\n  def test_raises_pass() do\n    Assert.assert_raises(fn -> raise \"boom\" end)\n  end\n\n  def test_no_raise_fails() do\n    Assert.assert_raises(fn -> 42 end)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test RaisesTest.test_raises_pass ... ok"),
        "raising function should pass, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test RaisesTest.test_no_raise_fails ... FAILED"),
        "non-raising function should fail, got:\n{stdout}"
    );
    assert!(
        stdout.contains("expected function to raise, but it returned normally"),
        "failure message should explain no raise, got:\n{stdout}"
    );
    assert!(
        stdout.contains("1 passed; 1 failed"),
        "summary should show 1 pass and 1 fail, got:\n{stdout}"
    );
}

#[test]
fn test_assert_raises_with_pattern_match() {
    let fixture_root = write_single_test_file(
        "test-assert-raises-pattern",
        "raises_pattern_test.tn",
        "defmodule RaisesPatternTest do\n  def test_matching_pattern() do\n    Assert.assert_raises(fn -> raise \"invalid argument\" end, \"invalid\")\n  end\n\n  def test_non_matching_pattern() do\n    Assert.assert_raises(fn -> raise \"something else\" end, \"invalid\")\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test RaisesPatternTest.test_matching_pattern ... ok"),
        "matching pattern should pass, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test RaisesPatternTest.test_non_matching_pattern ... FAILED"),
        "non-matching pattern should fail, got:\n{stdout}"
    );
    assert!(
        stdout.contains("expected raise matching"),
        "failure should mention expected pattern, got:\n{stdout}"
    );
}

#[test]
fn test_assert_raises_json_output() {
    let fixture_root = write_single_test_file(
        "test-assert-raises-json",
        "raises_json_test.tn",
        "defmodule RaisesJsonTest do\n  def test_raises_ok() do\n    Assert.assert_raises(fn -> raise \"kaboom\" end)\n  end\n\n  def test_raises_fail() do\n    Assert.assert_raises(fn -> :safe end)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args([
            "test",
            &fixture_root.display().to_string(),
            "--format",
            "json",
        ])
        .output()
        .expect("test command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let json: Value = serde_json::from_str(&stdout).expect("output should be valid JSON");

    assert_eq!(json["passed"], 1, "JSON should show 1 passed");
    assert_eq!(json["failed"], 1, "JSON should show 1 failed");

    let results = json["results"]
        .as_array()
        .expect("results should be an array");
    let passed = results
        .iter()
        .find(|r| r["status"] == "passed")
        .expect("should have a passed result");
    assert!(
        passed["id"].as_str().unwrap().contains("test_raises_ok"),
        "passed test should be test_raises_ok"
    );

    let failed = results
        .iter()
        .find(|r| r["status"] == "failed")
        .expect("should have a failed result");
    assert!(
        failed["id"].as_str().unwrap().contains("test_raises_fail"),
        "failed test should be test_raises_fail"
    );
    assert!(
        failed["error"]
            .as_str()
            .unwrap()
            .contains("expected function to raise"),
        "failed error should mention no raise"
    );
}

// ── assert_match tests ─────────────────────────────────────────────────────

#[test]
fn test_assert_match_map_subset_passes() {
    let fixture_root = write_single_test_file(
        "test-assert-match-subset-pass",
        "match_pass_test.tn",
        "defmodule MatchPassTest do\n  def test_subset_match() do\n    Assert.assert_match(%{a: 1}, %{a: 1, b: 2})\n  end\n\n  def test_exact_match() do\n    Assert.assert_match(%{a: 1, b: 2}, %{a: 1, b: 2})\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "subset match should pass, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("2 passed; 0 failed"));
}

#[test]
fn test_assert_match_map_subset_fails_on_mismatch() {
    let fixture_root = write_single_test_file(
        "test-assert-match-subset-fail",
        "match_fail_test.tn",
        "defmodule MatchFailTest do\n  def test_mismatched_value() do\n    Assert.assert_match(%{a: 2}, %{a: 1, b: 2})\n  end\n\n  def test_missing_key() do\n    Assert.assert_match(%{c: 3}, %{a: 1, b: 2})\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert!(!output.status.success(), "mismatch should fail");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("assert_match failed"),
        "should mention assert_match in failure output, got: {stdout}"
    );
    assert!(stdout.contains("0 passed; 2 failed"));
}

#[test]
fn test_assert_match_exact_equality_for_non_maps() {
    let fixture_root = write_single_test_file(
        "test-assert-match-non-map",
        "match_nonmap_test.tn",
        "defmodule MatchNonmapTest do\n  def test_equal_ints() do\n    Assert.assert_match(42, 42)\n  end\n\n  def test_unequal_ints() do\n    Assert.assert_match(42, 43)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args(["test", &fixture_root.display().to_string()])
        .output()
        .expect("test command should execute");

    assert!(!output.status.success(), "one test should fail");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("1 passed; 1 failed"),
        "should have 1 pass and 1 fail, got: {stdout}"
    );
}

// ── --timeout tests ────────────────────────────────────────────────────────

#[test]
fn test_timeout_marks_test_as_failed() {
    let fixture_root = write_single_test_file(
        "test-timeout-fail",
        "timeout_test.tn",
        "defmodule TimeoutTest do\n  def test_slow() do\n    System.sleep_ms(2000)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args([
            "test",
            &fixture_root.display().to_string(),
            "--timeout",
            "200",
        ])
        .output()
        .expect("test command should execute");

    assert!(!output.status.success(), "timed-out test should fail");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("timed out after 200ms"),
        "should mention timeout in output, got: {stdout}"
    );
    assert!(
        stdout.contains("FAILED"),
        "should show FAILED status, got: {stdout}"
    );
}

#[test]
fn test_timeout_does_not_affect_fast_tests() {
    let fixture_root = write_single_test_file(
        "test-timeout-fast",
        "fast_test.tn",
        "defmodule FastTest do\n  def test_quick() do\n    1 + 1\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args([
            "test",
            &fixture_root.display().to_string(),
            "--timeout",
            "5000",
        ])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "fast test with generous timeout should pass, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("1 passed; 0 failed"),
        "should pass normally, got: {stdout}"
    );
}

#[test]
fn test_timeout_json_output() {
    let fixture_root = write_single_test_file(
        "test-timeout-json",
        "timeout_json_test.tn",
        "defmodule TimeoutJsonTest do\n  def test_hangs() do\n    System.sleep_ms(2000)\n  end\n\n  def test_ok() do\n    42\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .env("NO_COLOR", "1")
        .args([
            "test",
            &fixture_root.display().to_string(),
            "--timeout",
            "200",
            "--format",
            "json",
        ])
        .output()
        .expect("test command should execute");

    assert!(!output.status.success(), "one test should fail");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let json: Value = serde_json::from_str(&stdout).expect("should be valid JSON");
    assert_eq!(json["status"], "failed");
    assert_eq!(json["passed"], 1);
    assert_eq!(json["failed"], 1);

    // Find the timed-out result
    let results = json["results"].as_array().expect("results should be array");
    let timed_out = results
        .iter()
        .find(|r| r["status"] == "failed")
        .expect("should have a failed result");
    let error = timed_out["error"].as_str().expect("should have error");
    assert!(
        error.contains("timed out after 200ms"),
        "JSON error should mention timeout, got: {error}"
    );
}

// ── teardown/0 tests ──────────────────────────────────────────────────────

#[test]
fn test_teardown_function_runs_after_each_test() {
    // teardown/0 that succeeds — tests should still pass normally.
    // Also verifies teardown is NOT listed as a test itself.
    let fixture_root = write_single_test_file(
        "test-teardown-runs",
        "teardown_test.tn",
        "defmodule TeardownTest do
  def teardown() do
    :ok
  end

  def test_alpha() do
    :ok
  end

  def test_beta() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "teardown_test.tn"])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "all tests should pass with ok teardown"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test TeardownTest.test_alpha ... ok"),
        "alpha should pass, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test TeardownTest.test_beta ... ok"),
        "beta should pass, got:\n{stdout}"
    );
    assert!(
        stdout.contains("2 passed; 0 failed; 2 total"),
        "summary should show 2 passed, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("test TeardownTest.teardown"),
        "teardown should not be listed as a test, got:\n{stdout}"
    );
}

#[test]
fn test_teardown_runs_even_after_test_failure() {
    // Test fails, but teardown still runs (succeeds). The test should still be marked failed
    // with the original assertion error, not a teardown error.
    let fixture_root = write_single_test_file(
        "test-teardown-after-fail",
        "teardown_after_fail_test.tn",
        "defmodule TeardownAfterFailTest do
  def teardown() do
    :ok
  end

  def test_fails() do
    Assert.assert(false, \"intentional failure\")
  end

  def test_passes() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "teardown_after_fail_test.tn"])
        .output()
        .expect("test command should execute");

    assert!(!output.status.success(), "one test should fail");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test TeardownAfterFailTest.test_fails ... FAILED"),
        "failing test should be marked FAILED, got:\n{stdout}"
    );
    assert!(
        stdout.contains("test TeardownAfterFailTest.test_passes ... ok"),
        "passing test should still pass, got:\n{stdout}"
    );
    assert!(
        stdout.contains("intentional failure"),
        "original error should be preserved, got:\n{stdout}"
    );
    // Teardown succeeded, so no "teardown failed" message
    assert!(
        !stdout.contains("teardown failed"),
        "teardown succeeded so no teardown error, got:\n{stdout}"
    );
}

#[test]
fn test_teardown_failure_marks_test_as_failed() {
    // teardown/0 returns err(...) — even passing tests should be marked as failed.
    let fixture_root = write_single_test_file(
        "test-teardown-failure",
        "teardown_failure_test.tn",
        "defmodule TeardownFailTest do
  def teardown() do
    err(:cleanup_broken)
  end

  def test_alpha() do
    :ok
  end
end
",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("NO_COLOR", "1")
        .args(["test", "teardown_failure_test.tn"])
        .output()
        .expect("test command should execute");

    assert!(
        !output.status.success(),
        "should fail when teardown returns err"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("test TeardownFailTest.test_alpha ... FAILED"),
        "test should be marked FAILED due to teardown, got:\n{stdout}"
    );
    assert!(
        stdout.contains("teardown failed"),
        "error should mention teardown failed, got:\n{stdout}"
    );
}
