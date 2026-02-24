mod common;

use std::process::Command;

#[test]
fn compiled_llvm_runtime_matches_catalog_for_core_values_and_collections() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("runtime-llvm-core-values-collections");

    for (fixture, expected_stdout) in fixture_contracts() {
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

fn fixture_contracts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("examples/parity/01-literals/atom_expression.tn", ":ok\n"),
        (
            "examples/parity/01-literals/bool_nil_string.tn",
            "{true, {false, {nil, \"hello\"}}}\n",
        ),
        (
            "examples/parity/01-literals/float_and_int.tn",
            "{1, 3.14}\n",
        ),
        (
            "examples/parity/02-operators/arithmetic_basic.tn",
            "{3, {2, {8, 5}}}\n",
        ),
        (
            "examples/parity/02-operators/comparison_set.tn",
            "{true, {true, {true, {true, {true, true}}}}}\n",
        ),
        (
            "examples/parity/02-operators/membership_and_range.tn",
            "{true, {false, true}}\n",
        ),
        (
            "examples/parity/03-collections/list_literal.tn",
            "[1, 2, 3]\n",
        ),
        (
            "examples/parity/03-collections/keyword_literal_single_entry.tn",
            "[ok: 1]\n",
        ),
        (
            "examples/parity/03-collections/map_literal_single_entry.tn",
            "%{:ok => 1}\n",
        ),
        (
            "examples/parity/03-collections/map_update_single_key.tn",
            "%{:a => 2}\n",
        ),
        (
            "examples/parity/03-collections/map_dot_and_index_access.tn",
            "{42, 42}\n",
        ),
        (
            "examples/parity/99-stretch/bitstring_binary.tn",
            "[1, 2, 3]\n",
        ),
        (
            "examples/parity/99-stretch/multi_entry_map_literal.tn",
            "%{:a => 1, :b => 2}\n",
        ),
        (
            "examples/parity/99-stretch/multi_entry_keyword_literal.tn",
            "[a: 1, b: 2]\n",
        ),
    ]
}
