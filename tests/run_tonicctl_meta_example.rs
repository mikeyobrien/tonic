use std::process::Command;

mod common;

#[test]
fn run_tonicctl_example_emits_expected_plan_sections() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&repo_root)
        .args(["run", "examples/apps/tonicctl"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected tonicctl example to run successfully, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    for needle in [
        ":tool => :tonicctl",
        ":mode => :plan_only",
        ":doctor",
        ":gates",
        ":bench_strict",
        ":release_dry_run",
    ] {
        assert!(
            stdout.contains(needle),
            "expected tonicctl plan output to contain '{needle}', got: {stdout}"
        );
    }
}

#[test]
fn compile_tonicctl_example_outputs_runnable_plan_binary() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_dir = common::unique_temp_dir("compile-tonicctl-meta-example");
    let out_path = temp_dir.join("tonicctl-plan");

    let compile_output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&repo_root)
        .args([
            "compile",
            "examples/apps/tonicctl",
            "--out",
            out_path.to_str().expect("out path should be utf8"),
        ])
        .output()
        .expect("compile command should execute");

    assert!(
        compile_output.status.success(),
        "expected tonicctl compile success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        compile_output.status.code(),
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );

    assert!(
        out_path.exists(),
        "expected compiled tonicctl binary at {}",
        out_path.display()
    );

    let run_output = Command::new(&out_path)
        .output()
        .expect("compiled tonicctl binary should execute");

    assert!(
        run_output.status.success(),
        "expected compiled tonicctl binary to succeed, got status {:?} and stderr: {}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let stdout = String::from_utf8(run_output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains(":tool => :tonicctl"),
        "expected compiled tonicctl output to include tool marker, got: {stdout}"
    );
}
