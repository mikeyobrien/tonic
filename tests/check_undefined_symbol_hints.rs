use std::fs;
mod common;

#[test]
fn check_reports_imported_call_typo_with_imported_hint() {
    let fixture_root = common::unique_fixture_root("check-undefined-symbol-imported-typo");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_imported_call_typo.tn"),
        "defmodule Math do\n  def helper(value) do\n    value\n  end\nend\n\ndefmodule Demo do\n  import Math\n\n  def run() do\n    halper(1)\n  end\nend\n",
    )
    .expect("fixture setup should write imported typo source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_imported_call_typo.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for imported call typo, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E1001] undefined symbol 'halper' in Demo.run; did you mean `helper/1` from imported module `Math`?"
        ),
        "unexpected imported-typo diagnostic: {stderr}"
    );
    assert!(stderr.contains("--> examples/resolver_imported_call_typo.tn:11:5"));
    assert!(stderr.contains("11 |     halper(1)"));
}

#[test]
fn check_reports_missing_import_guidance_for_unqualified_calls() {
    let fixture_root = common::unique_fixture_root("check-undefined-symbol-missing-import");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_missing_import_hint.tn"),
        "defmodule Math do\n  def helper(value) do\n    value\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    helper(1)\n  end\nend\n",
    )
    .expect("fixture setup should write missing import source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_missing_import_hint.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for missing import guidance case, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E1001] undefined symbol 'helper' in Demo.run; hint: call `Math.helper/1` or add `import Math` to use `helper/1` here"
        ),
        "unexpected missing-import diagnostic: {stderr}"
    );
    assert!(stderr.contains("--> examples/resolver_missing_import_hint.tn:9:5"));
    assert!(stderr.contains("9 |     helper(1)"));
}

#[test]
fn check_reports_module_qualified_call_typos_with_available_functions() {
    let fixture_root = common::unique_fixture_root("check-undefined-symbol-qualified-typo");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_qualified_call_typo.tn"),
        "defmodule Math do\n  def helper() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.helpr()\n  end\nend\n",
    )
    .expect("fixture setup should write qualified typo source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_qualified_call_typo.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for qualified call typo, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E1001] undefined symbol 'Math.helpr' in Demo.run; did you mean `Math.helper/0`?. Available Math functions: helper"
        ),
        "unexpected qualified-typo diagnostic: {stderr}"
    );
    assert!(stderr.contains("--> examples/resolver_qualified_call_typo.tn:9:5"));
    assert!(stderr.contains("9 |     Math.helpr()"));
}
