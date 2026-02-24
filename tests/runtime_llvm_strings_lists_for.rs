mod common;

use std::process::Command;

#[test]
fn compiled_llvm_runtime_matches_catalog_for_interpolation_and_concat_list() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("runtime-llvm-strings-lists");

    for (fixture, expected_stdout) in success_fixture_contracts() {
        let source = repo_root.join(fixture);
        assert!(source.exists(), "expected fixture {fixture} to exist");

        let compile = Command::new(env!("CARGO_BIN_EXE_tonic"))
            .current_dir(&temp_dir)
            .args([
                "compile",
                source.to_str().expect("fixture path should be utf8"),
                "--backend",
                "llvm",
            ])
            .output()
            .expect("compile command should execute");

        assert!(
            compile.status.success(),
            "expected llvm compile success for {fixture}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
            compile.status.code(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        );

        let compile_stdout = String::from_utf8_lossy(&compile.stdout);
        let artifact = compile_stdout
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("compile: ok ").map(str::trim))
            .expect("compile stdout should include artifact path");

        let executable_path = if std::path::Path::new(artifact).is_absolute() {
            std::path::PathBuf::from(artifact)
        } else {
            temp_dir.join(artifact)
        };

        let runtime = Command::new(&executable_path)
            .current_dir(&temp_dir)
            .output()
            .expect("compiled executable should run");

        assert!(
            runtime.status.success(),
            "expected runtime success for {fixture}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
            runtime.status.code(),
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );

        let runtime_stdout = String::from_utf8_lossy(&runtime.stdout);
        assert_eq!(
            runtime_stdout, expected_stdout,
            "runtime stdout mismatch for {fixture}"
        );
    }
}

fn success_fixture_contracts() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "examples/parity/01-literals/interpolation_basic.tn",
            "\"hello 3\"\n",
        ),
        (
            "examples/parity/02-operators/concat_and_list_ops.tn",
            "{\"hello world\", {[1, 2, 3, 4], [1, 3]}}\n",
        ),
    ]
}
