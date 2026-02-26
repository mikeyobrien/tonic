mod common;

use std::collections::BTreeMap;
use std::fs;

#[test]
fn rc_mode_reclaims_acyclic_intermediates_and_reports_cycle_caveat() {
    let fixture_root = common::unique_temp_dir("runtime-memory-rc-smoke");
    let source_path = fixture_root.join("memory_rc.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    case [\n      [1, 2, 3, 4, 5, 6],\n      [7, 8, 9, 10, 11, 12],\n      [13, 14, 15, 16, 17, 18],\n      [19, 20, 21, 22, 23, 24],\n      [25, 26, 27, 28, 29, 30],\n      [31, 32, 33, 34, 35, 36],\n      [37, 38, 39, 40, 41, 42],\n      [43, 44, 45, 46, 47, 48],\n      [49, 50, 51, 52, 53, 54],\n      [55, 56, 57, 58, 59, 60],\n      [61, 62, 63, 64, 65, 66],\n      [67, 68, 69, 70, 71, 72]\n    ] do\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let compile_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "memory_rc.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        compile_output.status.success(),
        "compile should succeed, stderr: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let exe_path = fixture_root.join(".tonic/build/memory_rc");
    assert!(exe_path.exists(), "compiled executable should exist");

    let baseline_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_STATS", "1")
        .env("TONIC_MEMORY_MODE", "append_only")
        .output()
        .expect("baseline executable should run");
    assert!(
        baseline_run.status.success(),
        "baseline executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&baseline_run.stderr)
    );

    let rc_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_STATS", "1")
        .env("TONIC_MEMORY_MODE", "rc")
        .output()
        .expect("rc executable should run");
    assert!(
        rc_run.status.success(),
        "rc executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&rc_run.stderr)
    );

    let baseline_stdout =
        String::from_utf8(baseline_run.stdout).expect("baseline stdout should be utf8");
    let rc_stdout = String::from_utf8(rc_run.stdout).expect("rc stdout should be utf8");
    assert_eq!(
        rc_stdout, baseline_stdout,
        "rc mode must preserve program output"
    );

    let baseline_fields = parse_stats_fields(&baseline_run.stderr);
    let rc_fields = parse_stats_fields(&rc_run.stderr);

    assert_eq!(
        rc_fields.get("memory_mode").map(String::as_str),
        Some("rc"),
        "rc mode should report memory_mode=rc"
    );
    assert_eq!(
        rc_fields.get("cycle_collection").map(String::as_str),
        Some("off"),
        "rc mode should report cycle_collection=off caveat"
    );

    let baseline_heap_slots = parse_u64_field(&baseline_fields, "heap_slots");
    let rc_reclaims_total = parse_u64_field(&rc_fields, "reclaims_total");
    let rc_heap_live_slots = parse_u64_field(&rc_fields, "heap_live_slots");

    assert!(
        rc_reclaims_total > 0,
        "rc mode should reclaim acyclic temporary graphs"
    );
    assert!(
        rc_heap_live_slots < baseline_heap_slots,
        "rc mode should leave fewer live heap slots than append-only baseline"
    );
}

fn parse_stats_fields(stderr: &[u8]) -> BTreeMap<String, String> {
    let stderr = String::from_utf8(stderr.to_vec()).expect("stderr should be utf8");
    let stats_line = stderr
        .lines()
        .find(|line| line.starts_with("memory.stats c_runtime "))
        .expect("memory stats line should be emitted when TONIC_MEMORY_STATS=1");

    stats_line
        .split_whitespace()
        .skip(2)
        .filter_map(|token| token.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn parse_u64_field(fields: &BTreeMap<String, String>, key: &str) -> u64 {
    fields
        .get(key)
        .unwrap_or_else(|| panic!("expected key '{key}' in stats line"))
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("expected numeric value for key '{key}'"))
}
