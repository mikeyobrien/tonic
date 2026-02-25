use std::fs;
mod common;

#[test]
fn check_rejects_host_call_with_non_atom_key() {
    let fixture_root = common::unique_fixture_root("check-host-call-non-atom-key");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("host_call_non_atom_key.tn"),
        "defmodule Demo do\n  def run() do\n    host_call(1, 2)\n  end\nend\n",
    )
    .expect("fixture setup should write host_call mismatch source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/host_call_non_atom_key.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for host_call non-atom key, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert!(stderr.contains("error: [E2001] type mismatch: expected atom, found int at offset 47"));
    assert!(stderr.contains("--> line 3, column 15"));
    assert!(stderr.contains("3 |     host_call(1, 2)"));
}
