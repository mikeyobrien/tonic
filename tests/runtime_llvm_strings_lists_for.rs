mod common;

use std::process::Command;

#[test]
fn compiled_llvm_runtime_matches_catalog_for_interpolation_and_concat_list() {
    for contract in interpolation_and_concat_list_contracts() {
        assert_fixture_runtime(contract);
    }
}

#[test]
fn compiled_llvm_runtime_matches_catalog_for_generators() {
    for contract in for_generator_contracts() {
        assert_fixture_runtime(contract);
    }
}

struct RuntimeFixtureContract {
    fixture: &'static str,
    expected_exit: i32,
    expected_stdout: Option<&'static str>,
    expected_stderr_contains: Option<&'static str>,
}

fn assert_fixture_runtime(contract: RuntimeFixtureContract) {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("runtime-llvm-strings-lists");

    let source = repo_root.join(contract.fixture);
    assert!(
        source.exists(),
        "expected fixture {} to exist",
        contract.fixture
    );

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
        "expected llvm compile success for {}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
        contract.fixture,
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

    let runtime_exit = runtime.status.code().unwrap_or(-1);
    assert_eq!(
        runtime_exit,
        contract.expected_exit,
        "runtime exit mismatch for {}\nstdout:\n{}\nstderr:\n{}",
        contract.fixture,
        String::from_utf8_lossy(&runtime.stdout),
        String::from_utf8_lossy(&runtime.stderr)
    );

    if let Some(expected_stdout) = contract.expected_stdout {
        let runtime_stdout = String::from_utf8_lossy(&runtime.stdout);
        assert_eq!(
            runtime_stdout, expected_stdout,
            "runtime stdout mismatch for {}",
            contract.fixture
        );
    }

    if let Some(expected_stderr_contains) = contract.expected_stderr_contains {
        let runtime_stderr = String::from_utf8_lossy(&runtime.stderr);
        assert!(
            runtime_stderr.contains(expected_stderr_contains),
            "runtime stderr mismatch for {}\nexpected to contain: {}\nactual stderr:\n{}",
            contract.fixture,
            expected_stderr_contains,
            runtime_stderr
        );
    }
}

fn interpolation_and_concat_list_contracts() -> Vec<RuntimeFixtureContract> {
    vec![
        RuntimeFixtureContract {
            fixture: "examples/parity/01-literals/interpolation_basic.tn",
            expected_exit: 0,
            expected_stdout: Some("\"hello 3\"\n"),
            expected_stderr_contains: None,
        },
        RuntimeFixtureContract {
            fixture: "examples/parity/02-operators/concat_and_list_ops.tn",
            expected_exit: 0,
            expected_stdout: Some("{\"hello world\", {[1, 2, 3, 4], [1, 3]}}\n"),
            expected_stderr_contains: None,
        },
    ]
}

fn for_generator_contracts() -> Vec<RuntimeFixtureContract> {
    vec![
        RuntimeFixtureContract {
            fixture: "examples/parity/06-control-flow/for_single_generator.tn",
            expected_exit: 0,
            expected_stdout: Some("[2, 4, 6]\n"),
            expected_stderr_contains: None,
        },
        RuntimeFixtureContract {
            fixture: "examples/parity/06-control-flow/for_multi_generator.tn",
            expected_exit: 0,
            expected_stdout: Some("[{1, 3}, {1, 4}, {2, 3}, {2, 4}]\n"),
            expected_stderr_contains: None,
        },
        RuntimeFixtureContract {
            fixture: "examples/parity/06-control-flow/for_into.tn",
            expected_exit: 0,
            expected_stdout: Some("[0, 2, 4]\n"),
            expected_stderr_contains: None,
        },
        RuntimeFixtureContract {
            fixture: "examples/parity/06-control-flow/for_into_runtime_fail.tn",
            expected_exit: 1,
            expected_stdout: Some(""),
            expected_stderr_contains: Some("error: for into destination must be a list, found map"),
        },
    ]
}
