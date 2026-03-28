use std::fs;
mod common;

#[test]
fn check_dump_ast_supports_string_interpolation() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-string-interpolation");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("interpolation.tn"),
        "defmodule Demo do\n  def run() do\n    \"hello #{1 + 2} world\"\n  end\nend\n",
    )
    .expect("fixture setup should write interpolation source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/interpolation.tn", "--dump-ast"])
        .output()
        .expect("check command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("interpolatedstring"));
    assert!(stdout.contains("segments"));
    assert!(stdout.contains("\"value\":\"hello \""));
    assert!(stdout.contains("\"value\":\" world\""));
}

#[test]
fn check_dump_ast_supports_interpolated_text_blocks() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-text-block-interpolation");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("text_block_interpolation.tn"),
        concat!(
            "defmodule Demo do\n",
            "  def run() do\n",
            "    name = \"Tonic\"\n",
            "    suffix = String.upcase(\n",
            "      \"ok\"\n",
            "    )\n",
            "    ~t\"\"\"\n",
            "      hello #{name}\n",
            "      result #{suffix}\n",
            "    \"\"\"\n",
            "  end\n",
            "end\n",
        ),
    )
    .expect("fixture setup should write interpolation source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args([
            "check",
            "examples/text_block_interpolation.tn",
            "--dump-ast",
        ])
        .output()
        .expect("check command should execute");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("interpolatedstring"));
    assert!(stdout.contains("\"value\":\"hello \""));
    assert!(stdout.contains("\"value\":\"\\nresult \""));
    assert!(stdout.contains("\"name\":\"name\""));
    assert!(stdout.contains("\"callee\":\"String.upcase\""));
}

#[test]
fn check_reports_unterminated_text_block_interpolation() {
    let fixture_root = common::unique_fixture_root("check-text-block-interpolation-diagnostic");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("bad_text_block_interpolation.tn"),
        concat!(
            "defmodule Demo do\n",
            "  def run() do\n",
            "    ~t\"\"\"\n",
            "      hello #{name\n",
            "    \"\"\"\n",
            "  end\n",
            "end\n",
        ),
    )
    .expect("fixture setup should write interpolation source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/bad_text_block_interpolation.tn"])
        .output()
        .expect("check command should execute");

    assert!(
        !output.status.success(),
        "expected failing check invocation for malformed interpolation"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("after interpolated expression"));
    assert!(stderr.contains("bad_text_block_interpolation.tn:8:1"));
}
