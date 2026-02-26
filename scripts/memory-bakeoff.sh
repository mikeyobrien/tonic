#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf '%s\n' "Usage: scripts/memory-bakeoff.sh [--ci]"
}

ci_mode=0
if [[ ${1:-} == "--ci" ]]; then
  ci_mode=1
  shift
fi

if [[ $# -gt 0 ]]; then
  usage >&2
  exit 64
fi

tonic_bin="${TONIC_MEMORY_TONIC_BIN:-target/debug/tonic}"
fixture_dir="${TONIC_MEMORY_FIXTURE_DIR:-examples/memory}"
artifact_dir="${TONIC_MEMORY_BAKEOFF_ARTIFACT_DIR:-.tonic/memory-bakeoff}"
raw_tsv="${TONIC_MEMORY_BAKEOFF_RAW_TSV:-$artifact_dir/raw.tsv}"
summary_tsv="${TONIC_MEMORY_BAKEOFF_SUMMARY_TSV:-$artifact_dir/summary.tsv}"
summary_md="${TONIC_MEMORY_BAKEOFF_SUMMARY_MD:-$artifact_dir/summary.md}"

if [[ -n "${TONIC_MEMORY_TIME_BIN:-}" ]]; then
  time_bin="${TONIC_MEMORY_TIME_BIN}"
elif [[ -x "/usr/bin/time" ]]; then
  time_bin="/usr/bin/time"
elif [[ -x "/run/current-system/sw/bin/time" ]]; then
  time_bin="/run/current-system/sw/bin/time"
else
  printf '%s\n' 'unable to locate GNU time binary; set TONIC_MEMORY_TIME_BIN' >&2
  exit 1
fi

if [[ -n "${TONIC_MEMORY_BAKEOFF_ITERATIONS:-}" ]]; then
  iterations="${TONIC_MEMORY_BAKEOFF_ITERATIONS}"
elif [[ "$ci_mode" -eq 1 ]]; then
  iterations="3"
else
  iterations="5"
fi

if ! [[ "$iterations" =~ ^[0-9]+$ ]] || [[ "$iterations" -lt 1 ]]; then
  printf 'invalid TONIC_MEMORY_BAKEOFF_ITERATIONS: %s\n' "$iterations" >&2
  exit 1
fi

scenarios=(
  "startup:startup_probe.tn"
  "throughput:map_growth_stress.tn"
  "cycle_churn:cycle_churn_stress.tn"
)
modes=(default append_only rc trace)

mkdir -p "$artifact_dir/bin" "$artifact_dir/logs"

if [[ ! -x "$tonic_bin" ]]; then
  printf 'Building tonic binary at %s...\n' "$tonic_bin"
  cargo build -q --bin tonic
fi

printf 'scenario\tmode\treported_mode\titeration\telapsed_us\trss_kb\treclaims_total\tgc_collections_total\theap_live_slots\n' >"$raw_tsv"

for scenario_entry in "${scenarios[@]}"; do
  scenario_name="${scenario_entry%%:*}"
  fixture_name="${scenario_entry#*:}"
  fixture_path="$fixture_dir/$fixture_name"

  if [[ ! -f "$fixture_path" ]]; then
    printf 'missing fixture: %s\n' "$fixture_path" >&2
    exit 1
  fi

  exe_path="$artifact_dir/bin/$scenario_name"
  compile_log="$artifact_dir/logs/$scenario_name.compile.log"
  "$tonic_bin" compile "$fixture_path" --out "$exe_path" >"$compile_log" 2>&1

  for mode in "${modes[@]}"; do
    for iteration in $(seq 1 "$iterations"); do
      stdout_log="$artifact_dir/logs/$scenario_name.$mode.$iteration.stdout.log"
      stderr_log="$artifact_dir/logs/$scenario_name.$mode.$iteration.stderr.log"
      time_log="$artifact_dir/logs/$scenario_name.$mode.$iteration.time.log"

      start_ns="$(date +%s%N)"
      if [[ "$mode" == "default" ]]; then
        "$time_bin" -f '%M' -o "$time_log" -- \
          env TONIC_MEMORY_STATS=1 "$exe_path" >"$stdout_log" 2>"$stderr_log"
      else
        "$time_bin" -f '%M' -o "$time_log" -- \
          env TONIC_MEMORY_STATS=1 TONIC_MEMORY_MODE="$mode" "$exe_path" >"$stdout_log" 2>"$stderr_log"
      fi
      end_ns="$(date +%s%N)"

      stats_line="$(grep -m1 '^memory.stats c_runtime ' "$stderr_log" || true)"
      if [[ -z "$stats_line" ]]; then
        printf 'missing memory stats line for scenario=%s mode=%s iteration=%s\n' \
          "$scenario_name" "$mode" "$iteration" >&2
        exit 1
      fi

      declare -A fields=()
      for token in $stats_line; do
        case "$token" in
          *=*)
            key="${token%%=*}"
            value="${token#*=}"
            fields["$key"]="$value"
            ;;
        esac
      done

      reported_mode="${fields[memory_mode]:-unknown}"
      reclaims_total="${fields[reclaims_total]:-0}"
      gc_collections_total="${fields[gc_collections_total]:-0}"
      heap_live_slots="${fields[heap_live_slots]:-0}"

      elapsed_us="$(( (end_ns - start_ns) / 1000 ))"
      rss_kb="$(tr -d '\r\n' < "$time_log")"

      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$scenario_name" \
        "$mode" \
        "$reported_mode" \
        "$iteration" \
        "$elapsed_us" \
        "$rss_kb" \
        "$reclaims_total" \
        "$gc_collections_total" \
        "$heap_live_slots" >>"$raw_tsv"
    done
  done
done

python3 - "$raw_tsv" "$summary_tsv" "$summary_md" "$ci_mode" <<'PY'
import csv
import math
import statistics
import sys
from collections import defaultdict

raw_tsv, summary_tsv, summary_md, ci_mode = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])


def percentile(values, p):
    if not values:
        return 0.0
    ordered = sorted(values)
    rank = max(0, math.ceil(p * len(ordered)) - 1)
    rank = min(rank, len(ordered) - 1)
    return ordered[rank]


def median_int(values):
    if not values:
        return 0
    return int(round(statistics.median(values)))


def median_float(values):
    if not values:
        return 0.0
    return float(statistics.median(values))

rows = []
with open(raw_tsv, newline="", encoding="utf-8") as handle:
    reader = csv.DictReader(handle, delimiter="\t")
    for row in reader:
        rows.append(
            {
                "scenario": row["scenario"],
                "mode": row["mode"],
                "reported_mode": row["reported_mode"],
                "elapsed_ms": float(row["elapsed_us"]) / 1000.0,
                "rss_kb": int(row["rss_kb"]),
                "reclaims_total": int(row["reclaims_total"]),
                "gc_collections_total": int(row["gc_collections_total"]),
                "heap_live_slots": int(row["heap_live_slots"]),
            }
        )

groups = defaultdict(list)
for row in rows:
    groups[(row["scenario"], row["mode"])].append(row)

summary_rows = []
for (scenario, mode), bucket in sorted(groups.items()):
    elapsed = [r["elapsed_ms"] for r in bucket]
    rss = [r["rss_kb"] for r in bucket]
    reclaims = [r["reclaims_total"] for r in bucket]
    gc_cols = [r["gc_collections_total"] for r in bucket]
    live_slots = [r["heap_live_slots"] for r in bucket]

    reported_mode = bucket[-1]["reported_mode"]
    summary_rows.append(
        {
            "scenario": scenario,
            "mode": mode,
            "reported_mode": reported_mode,
            "median_elapsed_ms": median_float(elapsed),
            "p95_elapsed_ms": percentile(elapsed, 0.95),
            "median_rss_kb": median_int(rss),
            "median_reclaims_total": median_int(reclaims),
            "median_gc_collections_total": median_int(gc_cols),
            "median_heap_live_slots": median_int(live_slots),
        }
    )

with open(summary_tsv, "w", encoding="utf-8", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t")
    writer.writerow(
        [
            "scenario",
            "mode",
            "reported_mode",
            "median_elapsed_ms",
            "p95_elapsed_ms",
            "median_rss_kb",
            "median_reclaims_total",
            "median_gc_collections_total",
            "median_heap_live_slots",
        ]
    )
    for row in summary_rows:
        writer.writerow(
            [
                row["scenario"],
                row["mode"],
                row["reported_mode"],
                f"{row['median_elapsed_ms']:.3f}",
                f"{row['p95_elapsed_ms']:.3f}",
                row["median_rss_kb"],
                row["median_reclaims_total"],
                row["median_gc_collections_total"],
                row["median_heap_live_slots"],
            ]
        )

with open(summary_md, "w", encoding="utf-8") as handle:
    handle.write("# Runtime memory bakeoff\n\n")
    handle.write("| scenario | mode | reported_mode | median_elapsed_ms | p95_elapsed_ms | median_rss_kb | median_reclaims_total | median_gc_collections_total | median_heap_live_slots |\n")
    handle.write("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |\n")
    for row in summary_rows:
        handle.write(
            "| {scenario} | {mode} | {reported_mode} | {median_elapsed_ms:.3f} | {p95_elapsed_ms:.3f} | {median_rss_kb} | {median_reclaims_total} | {median_gc_collections_total} | {median_heap_live_slots} |\n".format(
                **row
            )
        )

if ci_mode:
    index = {(row["scenario"], row["mode"]): row for row in summary_rows}
    failures = []

    startup_default = index.get(("startup", "default"))
    if startup_default is None or startup_default["reported_mode"] != "trace":
        failures.append("default mode must resolve to trace in startup scenario")

    cycle_append = index.get(("cycle_churn", "append_only"))
    cycle_trace = index.get(("cycle_churn", "trace"))
    if cycle_append is None or cycle_trace is None:
        failures.append("missing cycle_churn append_only/trace rows for guardrail checks")
    else:
        if cycle_trace["median_reclaims_total"] <= cycle_append["median_reclaims_total"]:
            failures.append("trace must reclaim more cycle churn objects than append_only")
        if cycle_trace["median_heap_live_slots"] >= cycle_append["median_heap_live_slots"]:
            failures.append("trace must retain fewer live slots than append_only under cycle churn")
        if cycle_trace["median_gc_collections_total"] <= 0:
            failures.append("trace must report at least one GC collection")

    startup_append = index.get(("startup", "append_only"))
    startup_trace = index.get(("startup", "trace"))
    if startup_append is not None and startup_trace is not None:
        threshold = startup_append["median_elapsed_ms"] * 3.0
        if startup_trace["median_elapsed_ms"] > threshold:
            failures.append(
                "trace startup median latency exceeds 3.0x append_only baseline"
            )

    if failures:
        for failure in failures:
            print(f"memory-bakeoff guardrail failed: {failure}", file=sys.stderr)
        raise SystemExit(1)
PY

printf 'memory bakeoff raw rows: %s\n' "$raw_tsv"
printf 'memory bakeoff summary: %s\n' "$summary_tsv"
printf 'memory bakeoff report: %s\n' "$summary_md"
if [[ "$ci_mode" -eq 1 ]]; then
  printf 'memory bakeoff guardrails: pass\n'
fi
