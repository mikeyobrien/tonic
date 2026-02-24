#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf '%s\n' "Usage: scripts/native-regression-policy.sh <summary.json> [--mode strict|advisory]"
}

if [[ $# -lt 1 || $# -gt 3 ]]; then
  usage >&2
  exit 64
fi

report_path="$1"
shift

mode="strict"
if [[ $# -gt 0 ]]; then
  if [[ "$1" != "--mode" || $# -ne 2 ]]; then
    usage >&2
    exit 64
  fi

  mode="$2"
  if [[ "$mode" != "strict" && "$mode" != "advisory" ]]; then
    usage >&2
    exit 64
  fi
fi

if [[ ! -f "$report_path" ]]; then
  printf 'summary report not found: %s\n' "$report_path" >&2
  exit 64
fi

python3 - "$report_path" "$mode" <<'PY'
import json
import math
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
mode = sys.argv[2]

try:
    report = json.loads(report_path.read_text())
except Exception as exc:  # pragma: no cover - shell contract
    print(f"verdict=rollback reason=invalid_report_json error={exc}")
    sys.exit(3)

contract = report.get("performance_contract")
if not isinstance(contract, dict):
    print("verdict=rollback reason=missing_performance_contract")
    sys.exit(3)

if contract.get("status") == "pass":
    score = float(contract.get("overall_score", 1.0))
    threshold = float(contract.get("pass_threshold", 0.0))
    print(f"verdict=pass score={score:.3f} threshold={threshold:.3f}")
    sys.exit(0)

relative_budget_pct = float(contract.get("relative_budget_pct", 0.0))
budget_ratio = 1.0 + (relative_budget_pct / 100.0)
quarantine_ratio = budget_ratio + 0.10
rollback_ratio = budget_ratio + 0.20

score = float(contract.get("overall_score", 0.0))
pass_threshold = float(contract.get("pass_threshold", 1.0))
score_gap = pass_threshold - score

slo = contract.get("slo") or {}
slo_status = slo.get("status", "unknown")
slo_failures = slo.get("failures") if isinstance(slo.get("failures"), list) else []

workload_scores = contract.get("workload_scores") if isinstance(contract.get("workload_scores"), list) else []

soft_regressions = 0
hard_regressions = 0

for workload in workload_scores:
    if not isinstance(workload, dict):
        continue

    ratios = [
        workload.get("p50_ratio_to_best_ref"),
        workload.get("p95_ratio_to_best_ref"),
        workload.get("rss_ratio_to_best_ref"),
    ]
    numeric_ratios = [value for value in ratios if isinstance(value, (int, float)) and math.isfinite(value)]
    if not numeric_ratios:
        continue

    if any(value > rollback_ratio for value in numeric_ratios):
        hard_regressions += 1
    elif any(value > budget_ratio for value in numeric_ratios):
        soft_regressions += 1

failure_reasons = contract.get("failure_reasons") if isinstance(contract.get("failure_reasons"), list) else []

if slo_status == "fail" or slo_failures or hard_regressions > 0 or score_gap > 0.08:
    print(
        "verdict=rollback"
        f" score={score:.3f}"
        f" threshold={pass_threshold:.3f}"
        f" soft_regressions={soft_regressions}"
        f" hard_regressions={hard_regressions}"
    )
    sys.exit(3)

if soft_regressions <= 2 and score_gap <= 0.03:
    print(
        "verdict=quarantine"
        f" score={score:.3f}"
        f" threshold={pass_threshold:.3f}"
        f" soft_regressions={soft_regressions}"
        f" hard_regressions={hard_regressions}"
    )
    if mode == "advisory":
        sys.exit(0)
    sys.exit(2)

summary = "; ".join(str(item) for item in failure_reasons[:2])
print(
    "verdict=rollback"
    f" score={score:.3f}"
    f" threshold={pass_threshold:.3f}"
    f" soft_regressions={soft_regressions}"
    f" hard_regressions={hard_regressions}"
    f" reasons={summary}"
)
sys.exit(3)
PY
