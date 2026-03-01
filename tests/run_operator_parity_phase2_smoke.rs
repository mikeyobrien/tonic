use std::fs;
mod common;

fn run_tonic_source(test_name: &str, source: &str) -> String {
    let fixture_root = common::unique_fixture_root(test_name);
    let examples_dir = fixture_root.join("examples");
    std::fs::create_dir_all(&examples_dir).unwrap();
    let source_path = examples_dir.join("main.tn");
    std::fs::write(&source_path, source).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/main.tn"])
        .output()
        .expect("tonic run should execute");

    assert!(
        output.status.success(),
        "run should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn parity_bitwise_and_operator() {
    let source = fs::read_to_string("examples/parity/02-operators/bitwise_operators.tn")
        .expect("bitwise_operators.tn should exist");
    let stdout = run_tonic_source("parity-bitwise-ops", &source);
    assert_eq!(stdout, "12\n");
}

#[test]
fn parity_div_rem_operators() {
    let source = fs::read_to_string("examples/parity/02-operators/div_rem.tn")
        .expect("div_rem.tn should exist");
    let stdout = run_tonic_source("parity-div-rem", &source);
    assert_eq!(stdout, "3\n1\n");
}

#[test]
fn parity_not_in_operator() {
    let source = fs::read_to_string("examples/parity/02-operators/not_in.tn")
        .expect("not_in.tn should exist");
    let stdout = run_tonic_source("parity-not-in", &source);
    assert_eq!(stdout, "true\nfalse\n");
}

#[test]
fn parity_stepped_range_operator() {
    let source = fs::read_to_string("examples/parity/02-operators/stepped_range.tn")
        .expect("stepped_range.tn should exist");
    let stdout = run_tonic_source("parity-stepped-range", &source);
    assert_eq!(stdout, "[1, 4, 7, 10]\n");
}
