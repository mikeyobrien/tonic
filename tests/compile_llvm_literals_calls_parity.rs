mod common;

use std::process::Command;

#[test]
fn compile_llvm_matches_catalog_for_literals_and_call_surface_fixtures() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("compile-llvm-literals-calls-parity");

    for fixture in fixture_paths() {
        let source = repo_root.join(fixture);
        assert!(
            source.exists(),
            "expected fixture to exist at {}",
            source.display()
        );

        let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
            .current_dir(&temp_dir)
            .args([
                "compile",
                source.to_str().expect("fixture path should be utf8"),
            ])
            .output()
            .expect("compile command should execute");

        assert!(
            output.status.success(),
            "expected llvm compile success for {fixture}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("compile: ok"),
            "expected compile success marker for {fixture}, got stdout:\n{stdout}"
        );
    }
}

fn fixture_paths() -> Vec<&'static str> {
    vec![
        "examples/bools_and_nils.tn",
        "examples/parity/01-literals/bool_nil_string.tn",
        "examples/parity/01-literals/float_and_int.tn",
        "examples/parity/01-literals/heredoc_multiline.tn",
        "examples/parity/01-literals/interpolation_basic.tn",
        "examples/parity/02-operators/concat_and_list_ops.tn",
        "examples/parity/03-collections/map_dot_and_index_access.tn",
        "examples/parity/08-errors/host_call_and_protocol_dispatch.tn",
        "examples/parity/08-errors/ok_err_constructors.tn",
        "examples/parity/08-errors/question_operator_err_bubble.tn",
        "examples/parity/99-stretch/sigils.tn",
    ]
}
