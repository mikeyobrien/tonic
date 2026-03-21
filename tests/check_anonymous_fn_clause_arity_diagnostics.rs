use std::fs;
mod common;

#[test]
fn check_reports_anonymous_function_clause_arity_mismatch_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-anonymous-fn-clause-arity-mismatch");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("anonymous_fn_clause_arity_mismatch.tn"),
        "defmodule Demo do\n  def run() do\n    fn value -> value; left, right -> left + right end\n  end\nend\n",
    )
    .expect("fixture setup should write anonymous fn source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/anonymous_fn_clause_arity_mismatch.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for mismatched anonymous fn clause arities"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0009] anonymous function clause arity mismatch: the first clause takes 1 parameter, but this clause takes 2 parameters. hint: make every clause in the same 'fn' use the same arity, for example `value -> ...`"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(stderr.contains(" --> examples/anonymous_fn_clause_arity_mismatch.tn:3:24"));
    assert!(stderr.contains("3 |     fn value -> value; left, right -> left + right end"));
}
