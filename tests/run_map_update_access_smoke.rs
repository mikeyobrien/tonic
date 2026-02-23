use std::fs;
mod common;

#[test]
fn map_update_and_access_smoke_test() {
    let fixture_root = common::unique_fixture_root("run-map-update-access");
    let src_dir = fixture_root.join("examples");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");

    let source = r#"
defmodule Demo do
  def run() do
    do_test(%{a: 1})
  end

  def do_test(base) do
    do_check(%{base | a: 2})
  end

  def do_check(updated) do
    updated.a + updated[:a]
  end
end
"#;

    fs::write(src_dir.join("map_access.tn"), source)
        .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/map_access.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "4\n");
}
