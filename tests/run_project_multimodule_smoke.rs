use std::fs;
mod common;

#[test]
fn run_executes_project_entry_with_sibling_module_dependencies() {
    let fixture_root = common::unique_fixture_root("run-project-multimodule-smoke");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n",
    )
    .expect("fixture setup should write entry module source");
    fs::write(
        src_dir.join("math.tn"),
        "defmodule Math do\n  def helper() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write sibling module source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful project run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "1\n");
}
