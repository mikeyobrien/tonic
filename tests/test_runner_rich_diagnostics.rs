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

fn write_single_test_file(test_name: &str, file_name: &str, source: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);

    fs::write(fixture_root.join(file_name), source).expect("fixture setup should write test file");

    fixture_root
}
