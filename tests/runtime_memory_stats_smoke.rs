mod common;

use std::collections::BTreeMap;
use std::fs;

#[test]
fn compiled_binary_emits_memory_stats_when_env_flag_enabled() {
    let fixture_root = common::unique_temp_dir("runtime-memory-stats-smoke");
    let source_path = fixture_root.join("memory_stats.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    for x <- [1, 2, 3, 4] do\n      [x, x + 1, x + 2]\n    end\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let compile_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "memory_stats.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        compile_output.status.success(),
        "compile should succeed, stderr: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let exe_path = fixture_root.join(".tonic/build/memory_stats");
    assert!(exe_path.exists(), "compiled executable should exist");

    let run_output = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_STATS", "1")
        .output()
        .expect("compiled executable should run");

    assert!(
        run_output.status.success(),
        "compiled executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&run_output.stderr)
    );

    let stderr = String::from_utf8(run_output.stderr).expect("stderr should be utf8");
    let stats_line = stderr
        .lines()
        .find(|line| line.starts_with("memory.stats c_runtime "))
        .expect("memory stats line should be emitted when TONIC_MEMORY_STATS=1");

    let fields = stats_line
        .split_whitespace()
        .skip(2)
        .filter_map(|token| token.split_once('='))
        .collect::<BTreeMap<_, _>>();

    for key in [
        "objects_total",
        "heap_slots",
        "heap_slots_hwm",
        "heap_capacity",
        "heap_capacity_hwm",
    ] {
        let value = fields
            .get(key)
            .unwrap_or_else(|| panic!("expected key '{key}' in stats line: {stats_line}"));
        value
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("expected numeric value for key '{key}': {stats_line}"));
    }
}
