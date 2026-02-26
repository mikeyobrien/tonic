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

#[test]
fn trace_gc_runs_without_stats_env() {
    // Verify that trace GC collection is decoupled from TONIC_MEMORY_STATS:
    // the collector must run at process end whenever TONIC_MEMORY_MODE=trace,
    // even if TONIC_MEMORY_STATS is not set. The observable contract:
    //   1. process exits successfully (no crash from GC running without stats)
    //   2. no stats line on stderr (TONIC_MEMORY_STATS gate is respected)
    //   3. when stats ARE enabled, gc_collections_total > 0 (GC ran)
    let fixture_root = common::unique_temp_dir("runtime-memory-trace-no-stats");
    let source_path = fixture_root.join("trace_no_stats.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    host_call(:memory_cycle_churn, 200)\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let compile_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "trace_no_stats.tn"])
        .output()
        .expect("compile command should execute");
    assert!(
        compile_output.status.success(),
        "compile should succeed, stderr: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let exe_path = fixture_root.join(".tonic/build/trace_no_stats");
    assert!(exe_path.exists(), "compiled executable should exist");

    // Run with trace mode but WITHOUT TONIC_MEMORY_STATS.
    let no_stats_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_MODE", "trace")
        .env_remove("TONIC_MEMORY_STATS")
        .output()
        .expect("no-stats trace executable should run");
    assert!(
        no_stats_run.status.success(),
        "trace mode without TONIC_MEMORY_STATS should exit successfully, stderr: {}",
        String::from_utf8_lossy(&no_stats_run.stderr)
    );
    let no_stats_stderr = String::from_utf8(no_stats_run.stderr).expect("stderr should be utf8");
    assert!(
        !no_stats_stderr
            .lines()
            .any(|l| l.starts_with("memory.stats c_runtime ")),
        "no stats line should be emitted when TONIC_MEMORY_STATS is unset; got: {no_stats_stderr}"
    );

    // Run with trace mode AND TONIC_MEMORY_STATS=1 to confirm GC ran (and
    // was recorded) — proving the GC path is active.
    let stats_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_MODE", "trace")
        .env("TONIC_MEMORY_STATS", "1")
        .output()
        .expect("stats trace executable should run");
    assert!(
        stats_run.status.success(),
        "trace mode with TONIC_MEMORY_STATS=1 should exit successfully"
    );
    let stats_fields = parse_stats_fields(&stats_run.stderr);
    let gc_collections = parse_u64_field(&stats_fields, "gc_collections_total");
    assert!(
        gc_collections > 0,
        "gc_collections_total should be > 0 confirming trace GC ran before stats output"
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
