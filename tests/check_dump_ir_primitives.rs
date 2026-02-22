use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_ir_matches_primitive_literal_lowering_snapshot() {
    let fixture_root = unique_fixture_root("check-dump-ir-primitives");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("ir_primitives.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(true, tuple(false, tuple(nil, \"hello world\")))\n  end\nend\n",
    )
    .expect("fixture setup should write primitive ir source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/ir_primitives.tn", "--dump-ir"])
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
        "{\"op\":\"const_bool\",\"value\":true,\"offset\":43},",
        "{\"op\":\"const_bool\",\"value\":false,\"offset\":55},",
        "{\"op\":\"const_nil\",\"offset\":68},",
        "{\"op\":\"const_string\",\"value\":\"hello world\",\"offset\":73},",
        "{\"op\":\"call\",\"callee\":{\"kind\":\"builtin\",\"name\":\"tuple\"},\"argc\":2,\"offset\":62},",
        "{\"op\":\"call\",\"callee\":{\"kind\":\"builtin\",\"name\":\"tuple\"},\"argc\":2,\"offset\":49},",
        "{\"op\":\"call\",\"callee\":{\"kind\":\"builtin\",\"name\":\"tuple\"},\"argc\":2,\"offset\":37},",
        "{\"op\":\"return\",\"offset\":37}",
        "]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ir_matches_float_literal_lowering_snapshot() {
    let fixture_root = unique_fixture_root("check-dump-ir-float");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("ir_float_literals.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(3.14, 0.5)\n  end\nend\n",
    )
    .expect("fixture setup should write float ir source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/ir_float_literals.tn", "--dump-ir"])
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
        "{\"op\":\"const_float\",\"value\":\"3.14\",\"offset\":43},",
        "{\"op\":\"const_float\",\"value\":\"0.5\",\"offset\":49},",
        "{\"op\":\"call\",\"callee\":{\"kind\":\"builtin\",\"name\":\"tuple\"},\"argc\":2,\"offset\":37},",
        "{\"op\":\"return\",\"offset\":37}",
        "]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ir_matches_collection_literal_lowering_snapshot() {
    let fixture_root = unique_fixture_root("check-dump-ir-collection-literals");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("ir_collection_literals.tn"),
        "defmodule Demo do\n  def run() do\n    tuple({1, 2}, tuple([3, 4], tuple(%{ok: 5}, [done: 6])))\n  end\nend\n",
    )
    .expect("fixture setup should write collection literal ir source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/ir_collection_literals.tn", "--dump-ir"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("\"name\":\"tuple\""));
    assert!(stdout.contains("\"name\":\"list\""));
    assert!(stdout.contains("\"name\":\"map\""));
    assert!(stdout.contains("\"name\":\"keyword\""));
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
