mod common;

use std::collections::BTreeMap;
use std::fs;

#[test]
fn default_mode_uses_trace_and_append_only_remains_available_as_rollback() {
    let fixture_root = common::unique_temp_dir("runtime-memory-default-mode-smoke");
    let source_path = fixture_root.join("memory_default_mode.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    host_call(:memory_cycle_churn, 320)\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let compile_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "memory_default_mode.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        compile_output.status.success(),
        "compile should succeed, stderr: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let exe_path = fixture_root.join(".tonic/build/memory_default_mode");
    assert!(exe_path.exists(), "compiled executable should exist");

    let default_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_STATS", "1")
        .output()
        .expect("default executable should run");
    assert!(
        default_run.status.success(),
        "default executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&default_run.stderr)
    );

    let append_only_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_STATS", "1")
        .env("TONIC_MEMORY_MODE", "append_only")
        .output()
        .expect("append-only executable should run");
    assert!(
        append_only_run.status.success(),
        "append-only executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&append_only_run.stderr)
    );

    let default_stdout =
        String::from_utf8(default_run.stdout).expect("default stdout should be utf8");
    let append_only_stdout =
        String::from_utf8(append_only_run.stdout).expect("append-only stdout should be utf8");
    assert_eq!(
        default_stdout, append_only_stdout,
        "default trace mode and append-only rollback must preserve output"
    );

    let default_fields = parse_stats_fields(&default_run.stderr);
    let append_only_fields = parse_stats_fields(&append_only_run.stderr);

    assert_eq!(
        default_fields.get("memory_mode").map(String::as_str),
        Some("trace"),
        "default mode should report memory_mode=trace"
    );
    assert_eq!(
        default_fields.get("cycle_collection").map(String::as_str),
        Some("mark_sweep"),
        "default mode should report cycle_collection=mark_sweep"
    );

    assert_eq!(
        append_only_fields.get("memory_mode").map(String::as_str),
        Some("append_only"),
        "rollback mode should report memory_mode=append_only"
    );
    assert_eq!(
        append_only_fields
            .get("cycle_collection")
            .map(String::as_str),
        Some("off"),
        "append-only rollback mode should report cycle_collection=off"
    );

    let default_gc_collections = parse_u64_field(&default_fields, "gc_collections_total");
    let default_reclaims = parse_u64_field(&default_fields, "reclaims_total");
    let default_live_slots = parse_u64_field(&default_fields, "heap_live_slots");

    let append_only_gc_collections = parse_u64_field(&append_only_fields, "gc_collections_total");
    let append_only_reclaims = parse_u64_field(&append_only_fields, "reclaims_total");
    let append_only_live_slots = parse_u64_field(&append_only_fields, "heap_live_slots");

    assert!(
        default_gc_collections > 0,
        "default trace mode should report at least one collection"
    );
    assert!(
        default_reclaims > append_only_reclaims,
        "default trace mode should reclaim cyclic objects"
    );
    assert_eq!(
        append_only_gc_collections, 0,
        "append-only rollback should not run mark/sweep collections"
    );
    assert!(
        default_live_slots < append_only_live_slots,
        "default trace mode should leave fewer live slots than append-only rollback"
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
