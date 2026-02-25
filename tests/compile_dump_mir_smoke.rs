use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_dump_mir_is_rejected_as_unsupported_argument() {
    let temp_dir = common::unique_temp_dir("compile-dump-mir");
    let source_path = temp_dir.join("dump_mir.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "dump_mir.tn", "--dump-mir"])
        .assert()
        .failure()
        .stderr(contains("error: unexpected argument '--dump-mir'"));
}
