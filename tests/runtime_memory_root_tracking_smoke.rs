mod common;

use std::collections::BTreeMap;
use std::fs;

#[test]
fn root_tracking_boundaries_do_not_change_program_output() {
    let fixture_root = common::unique_temp_dir("runtime-memory-root-tracking");
    let source_path = fixture_root.join("memory_roots.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def bump_twice(fun, value) do\n    fun.(fun.(value))\n  end\n\n  def run() do\n    {bump_twice(fn value -> value + 1 end, 3),\n     {for x <- [1, 2, 3] do\n        [x, x + 1]\n      end,\n      host_call(:sum_ints, 1, 2, 3)}}\n  end\nend\n",
    )
    .expect("fixture source should be written");

    let compile_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "memory_roots.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        compile_output.status.success(),
        "compile should succeed, stderr: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    let exe_path = fixture_root.join(".tonic/build/memory_roots");
    assert!(exe_path.exists(), "compiled executable should exist");

    let baseline_run = std::process::Command::new(&exe_path)
        .output()
        .expect("compiled executable should run without memory stats");
    assert!(
        baseline_run.status.success(),
        "baseline executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&baseline_run.stderr)
    );

    let stats_run = std::process::Command::new(&exe_path)
        .env("TONIC_MEMORY_STATS", "1")
        .output()
        .expect("compiled executable should run with memory stats enabled");
    assert!(
        stats_run.status.success(),
        "stats executable should exit successfully, stderr: {}",
        String::from_utf8_lossy(&stats_run.stderr)
    );

    let baseline_stdout =
        String::from_utf8(baseline_run.stdout).expect("baseline stdout should be utf8");
    let stats_stdout = String::from_utf8(stats_run.stdout).expect("stats stdout should be utf8");
    assert_eq!(
        stats_stdout, baseline_stdout,
        "root tracking should not change observable program stdout"
    );

    assert!(
        baseline_stdout.contains("[[1, 2], [2, 3], [3, 4]]")
            && baseline_stdout.contains("5")
            && baseline_stdout.contains("6"),
        "expected function/closure/host/comprehension fixture output, got: {baseline_stdout}"
    );

    let stderr = String::from_utf8(stats_run.stderr).expect("stderr should be utf8");
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
        "roots_registered_total",
        "root_frames_hwm",
        "root_slots_hwm",
        "object_alloc_id_hwm",
    ] {
        let value = fields
            .get(key)
            .unwrap_or_else(|| panic!("expected key '{key}' in stats line: {stats_line}"));
        value
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("expected numeric value for key '{key}': {stats_line}"));
    }

    let roots_registered_total = fields
        .get("roots_registered_total")
        .expect("roots_registered_total should be present")
        .parse::<u64>()
        .expect("roots_registered_total should be numeric");
    let root_frames_hwm = fields
        .get("root_frames_hwm")
        .expect("root_frames_hwm should be present")
        .parse::<u64>()
        .expect("root_frames_hwm should be numeric");

    assert!(
        roots_registered_total > 0,
        "root registrations should be exercised by call/closure/host/comprehension fixture"
    );
    assert!(
        root_frames_hwm > 0,
        "root frame high-water should be non-zero when root frame boundaries are wired"
    );
}
