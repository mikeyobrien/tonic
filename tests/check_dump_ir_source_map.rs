use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_ir_includes_source_offsets_for_ops() {
    let fixture_root = unique_fixture_root("check-dump-ir-source-map");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("ir_source_map.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write ir source map source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/ir_source_map.tn", "--dump-ir"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let expected = concat!(
        "{\"functions\":[",
        "{\"name\":\"Demo.run\",\"params\":[],\"ops\":[",
        "{\"op\":\"const_int\",\"value\":1,\"offset\":37},",
        "{\"op\":\"return\",\"offset\":37}",
        "]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}
