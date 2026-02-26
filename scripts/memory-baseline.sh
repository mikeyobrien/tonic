#!/usr/bin/env bash
set -euo pipefail

tonic_bin="${TONIC_MEMORY_TONIC_BIN:-target/debug/tonic}"
fixture_dir="${TONIC_MEMORY_FIXTURE_DIR:-examples/memory}"
artifact_dir="${TONIC_MEMORY_ARTIFACT_DIR:-.tonic/memory-baseline}"
summary_tsv="${TONIC_MEMORY_SUMMARY_TSV:-$artifact_dir/baseline.tsv}"
summary_md="${TONIC_MEMORY_SUMMARY_MD:-$artifact_dir/baseline.md}"

fixtures=(
  "list_nesting_stress.tn"
  "map_growth_stress.tn"
  "closure_capture_stress.tn"
)

mkdir -p "$artifact_dir"

if [[ ! -x "$tonic_bin" ]]; then
  printf 'Building tonic binary at %s...\n' "$tonic_bin"
  cargo build -q
fi

printf 'fixture\tobjects_total\theap_slots\theap_slots_hwm\theap_capacity\theap_capacity_hwm\n' >"$summary_tsv"
printf '# Runtime memory baseline\n\n' >"$summary_md"
printf '| fixture | objects_total | heap_slots | heap_slots_hwm | heap_capacity | heap_capacity_hwm |\n' >>"$summary_md"
printf '| --- | ---: | ---: | ---: | ---: | ---: |\n' >>"$summary_md"

for fixture_name in "${fixtures[@]}"; do
  fixture_path="$fixture_dir/$fixture_name"
  if [[ ! -f "$fixture_path" ]]; then
    printf 'missing fixture: %s\n' "$fixture_path" >&2
    exit 1
  fi

  stem="${fixture_name%.tn}"
  exe_path="$artifact_dir/$stem"
  compile_log="$artifact_dir/$stem.compile.log"
  stdout_log="$artifact_dir/$stem.stdout.log"
  stderr_log="$artifact_dir/$stem.stderr.log"

  "$tonic_bin" compile "$fixture_path" --out "$exe_path" >"$compile_log" 2>&1
  TONIC_MEMORY_STATS=1 "$exe_path" >"$stdout_log" 2>"$stderr_log"

  stats_line="$(grep -m1 '^memory.stats c_runtime ' "$stderr_log" || true)"
  if [[ -z "$stats_line" ]]; then
    printf 'missing memory stats line for fixture %s\n' "$fixture_name" >&2
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

  objects_total="${fields[objects_total]:-0}"
  heap_slots="${fields[heap_slots]:-0}"
  heap_slots_hwm="${fields[heap_slots_hwm]:-0}"
  heap_capacity="${fields[heap_capacity]:-0}"
  heap_capacity_hwm="${fields[heap_capacity_hwm]:-0}"

  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$fixture_name" \
    "$objects_total" \
    "$heap_slots" \
    "$heap_slots_hwm" \
    "$heap_capacity" \
    "$heap_capacity_hwm" >>"$summary_tsv"

  printf '| %s | %s | %s | %s | %s | %s |\n' \
    "$fixture_name" \
    "$objects_total" \
    "$heap_slots" \
    "$heap_slots_hwm" \
    "$heap_capacity" \
    "$heap_capacity_hwm" >>"$summary_md"
done

printf 'memory baseline written: %s\n' "$summary_tsv"
printf 'memory baseline report: %s\n' "$summary_md"
