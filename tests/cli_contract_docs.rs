mod common;

use std::process::Command;

#[test]
fn docs_produces_output_for_documented_module() {
    let fixture_root = common::unique_fixture_root("cli-docs");
    let examples_dir = fixture_root.join("examples");

    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("documented.tn"),
        "@moduledoc \"A sample module.\"\ndefmodule Documented do\n  @doc \"Returns the answer.\"\n  def answer() do\n    42\n  end\nend\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["docs", "examples/documented.tn"])
        .output()
        .expect("tonic docs should run");

    assert!(
        output.status.success(),
        "docs should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Documented"),
        "expected module name in docs output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("A sample module."),
        "expected moduledoc in docs output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("answer/0"),
        "expected function signature in docs output, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Returns the answer."),
        "expected doc string in docs output, got:\n{stdout}"
    );
}

#[test]
fn docs_fails_on_nonexistent_file() {
    let fixture_root = common::unique_fixture_root("cli-docs-missing");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["docs", "examples/nonexistent.tn"])
        .output()
        .expect("tonic docs should run even for missing input");

    assert!(
        !output.status.success(),
        "docs should fail for missing input path"
    );
}
