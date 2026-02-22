use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_ir_matches_result_and_case_lowering_snapshot() {
    let fixture_root = unique_fixture_root("check-dump-ir-result-case");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("ir_result_case.tn"),
        "defmodule Demo do\n  def run() do\n    case ok(1)? do\n      :ok -> 2\n      _ -> 3\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write ir result/case source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/ir_result_case.tn", "--dump-ir"])
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
        "{\"op\":\"const_int\",\"value\":1,\"offset\":45},",
        "{\"op\":\"call\",\"callee\":{\"kind\":\"builtin\",\"name\":\"ok\"},\"argc\":1,\"offset\":42},",
        "{\"op\":\"question\",\"offset\":47},",
        "{\"op\":\"case\",\"branches\":[",
        "{\"pattern\":{\"kind\":\"atom\",\"value\":\"ok\"},\"ops\":[{\"op\":\"const_int\",\"value\":2,\"offset\":65}]},",
        "{\"pattern\":{\"kind\":\"wildcard\"},\"ops\":[{\"op\":\"const_int\",\"value\":3,\"offset\":78}]}",
        "],\"offset\":37},",
        "{\"op\":\"return\",\"offset\":37}",
        "]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ir_lowers_list_and_map_case_patterns() {
    let fixture_root = unique_fixture_root("check-dump-ir-list-map-case");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("ir_list_map_case.tn"),
        "defmodule Demo do\n  def subject() do\n    map(:ok, list(1, 2))\n  end\n\n  def run() do\n    case subject() do\n      [head, tail] -> head + tail\n      %{:ok -> [value, _]} -> value + 0\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write list/map case source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/ir_list_map_case.tn", "--dump-ir"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let branches = &json["functions"][1]["ops"][1]["branches"];

    assert_eq!(branches[0]["pattern"]["kind"], "list");
    assert_eq!(branches[0]["pattern"]["items"][0]["kind"], "bind");
    assert_eq!(branches[1]["pattern"]["kind"], "map");
    assert_eq!(branches[1]["pattern"]["entries"][0]["key"]["kind"], "atom");
    assert_eq!(
        branches[1]["pattern"]["entries"][0]["value"]["kind"],
        "list"
    );
}

#[test]
fn check_dump_ir_lowers_pin_guard_and_match_operator_forms() {
    let fixture_root = unique_fixture_root("check-dump-ir-pin-guard-match");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("ir_pin_guard_match.tn"),
        "defmodule Demo do\n  def run() do\n    case list(7, 8) do\n      [^expected, value] when value == 8 -> expected = value\n      _ -> 0\n    end\n  end\n\n  def expected() do\n    7\n  end\nend\n",
    )
    .expect("fixture setup should write pin/guard/match source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/ir_pin_guard_match.tn", "--dump-ir"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let branches = &json["functions"][0]["ops"][3]["branches"];

    assert_eq!(branches[0]["pattern"]["kind"], "list");
    assert_eq!(branches[0]["pattern"]["items"][0]["kind"], "pin");
    assert_eq!(branches[0]["guard_ops"][2]["op"], "cmp_int");
    assert_eq!(branches[0]["ops"][1]["op"], "match");
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
