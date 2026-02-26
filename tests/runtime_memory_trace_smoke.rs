mod common;

use std::collections::BTreeMap;
use std::fs;

#[test]
fn trace_mode_reclaims_cyclic_graphs_and_reports_mark_sweep_stats() {
    let fixture_root = common::unique_temp_dir("runtime-memory-trace-smoke");
    let source_path = fixture_root.join("memory_trace.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    host_call(:memory_cycle_churn, 400)\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let compile_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "memory_trace.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        compile_output.status.success(),
        "compile should succeed, stderr: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let exe_path = fixture_root.join(".tonic/build/memory_trace");
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

    let trace_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_STATS", "1")
        .env("TONIC_MEMORY_MODE", "trace")
        .output()
        .expect("trace executable should run");
    assert!(
        trace_run.status.success(),
        "trace executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&trace_run.stderr)
    );

    let baseline_stdout =
        String::from_utf8(baseline_run.stdout).expect("baseline stdout should be utf8");
    let trace_stdout = String::from_utf8(trace_run.stdout).expect("trace stdout should be utf8");
    assert_eq!(
        trace_stdout, baseline_stdout,
        "trace mode must preserve program output"
    );

    let baseline_fields = parse_stats_fields(&baseline_run.stderr);
    let trace_fields = parse_stats_fields(&trace_run.stderr);

    assert_eq!(
        trace_fields.get("memory_mode").map(String::as_str),
        Some("trace"),
        "trace mode should report memory_mode=trace"
    );
    assert_eq!(
        trace_fields.get("cycle_collection").map(String::as_str),
        Some("mark_sweep"),
        "trace mode should report cycle_collection=mark_sweep"
    );

    let baseline_reclaims_total = parse_u64_field(&baseline_fields, "reclaims_total");
    let baseline_heap_live_slots = parse_u64_field(&baseline_fields, "heap_live_slots");

    let trace_reclaims_total = parse_u64_field(&trace_fields, "reclaims_total");
    let trace_heap_live_slots = parse_u64_field(&trace_fields, "heap_live_slots");
    let trace_gc_collections_total = parse_u64_field(&trace_fields, "gc_collections_total");

    assert_eq!(
        baseline_reclaims_total, 0,
        "append-only baseline should not reclaim cyclic garbage"
    );
    assert!(
        trace_gc_collections_total > 0,
        "trace mode should report at least one mark/sweep collection"
    );
    assert!(
        trace_reclaims_total > 0,
        "trace mode should reclaim cyclic garbage produced by fixture"
    );
    assert!(
        trace_heap_live_slots < baseline_heap_live_slots,
        "trace mode should leave fewer live heap slots than append-only baseline"
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
