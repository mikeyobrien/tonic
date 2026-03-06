#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import random
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

RUN_SCHEMA_NAME = "tonic.observability.run"
ARTIFACT_SCHEMA_NAME = "tonic.observability.artifacts"
LATEST_SCHEMA_NAME = "tonic.observability.latest"
TASK_SCHEMA_NAME = "tonic.observability.task-run"
SCHEMA_VERSION = 1
STATE_FILE_NAME = "state.json"


@dataclass
class RunPaths:
    output_root: Path
    bundle_dir: Path
    events_path: Path
    state_path: Path
    summary_path: Path
    artifacts_path: Path


def now_utc() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def generate_id(prefix: str) -> str:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    return f"{prefix}_{stamp}_{random.randint(0, 0xFFFFFF):06x}"


def command_name(value: str) -> str:
    return value.replace(".sh", "").replace("_", "-")


def run_paths(output_root: str, run_id: str) -> RunPaths:
    root = Path(output_root)
    bundle_dir = root / "runs" / run_id
    return RunPaths(
        output_root=root,
        bundle_dir=bundle_dir,
        events_path=bundle_dir / "events.jsonl",
        state_path=bundle_dir / STATE_FILE_NAME,
        summary_path=bundle_dir / "summary.json",
        artifacts_path=bundle_dir / "artifacts.json",
    )


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def read_state(paths: RunPaths) -> dict[str, Any]:
    return json.loads(paths.state_path.read_text(encoding="utf-8"))


def write_state(paths: RunPaths, state: dict[str, Any]) -> None:
    ensure_parent(paths.state_path)
    paths.state_path.write_text(json.dumps(state, indent=2) + "\n", encoding="utf-8")


def append_json_line(path: Path, payload: dict[str, Any]) -> None:
    ensure_parent(path)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(payload, separators=(",", ":")) + "\n")


def stdout_kv(**values: str | None) -> None:
    for key, value in values.items():
        if value is None:
            value = ""
        print(f"{key}={value}")


def metadata_block(args: argparse.Namespace) -> dict[str, Any] | None:
    return None


def start_run(args: argparse.Namespace) -> int:
    run_id = args.run_id or generate_id("run")
    task_id = args.task_id or generate_id(f"task_{command_name(args.command)}")
    paths = run_paths(args.output_root, run_id)
    paths.bundle_dir.mkdir(parents=True, exist_ok=True)

    argv = list(args.argv_item or [])
    if not argv and getattr(args, "argv", None):
        argv = list(args.argv)

    state = {
        "schema_name": RUN_SCHEMA_NAME,
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "task_id": task_id,
        "parent_run_id": args.parent_run_id or None,
        "tool": {
            "kind": args.tool_kind,
            "name": args.tool_name,
            "command": args.command,
        },
        "cwd": args.cwd,
        "worktree_root": args.worktree_root,
        "argv": argv,
        "target_path": None,
        "command_metadata": metadata_block(args),
        "started_at": args.started_at or now_utc(),
        "phases": [],
        "artifacts": [],
    }
    write_state(paths, state)
    append_json_line(
        paths.events_path,
        {
            "type": "run.started",
            "run_id": run_id,
            "task_id": task_id,
            "parent_run_id": args.parent_run_id or None,
            "command": args.command,
            "argv": argv,
            "cwd": args.cwd,
            "at": state["started_at"],
        },
    )
    stdout_kv(
        run_id=run_id,
        task_id=task_id,
        parent_run_id=args.parent_run_id or "",
        output_root=str(paths.output_root),
    )
    return 0


def start_step(args: argparse.Namespace) -> int:
    paths = run_paths(args.output_root, args.run_id)
    append_json_line(
        paths.events_path,
        {
            "type": "step.started",
            "run_id": args.run_id,
            "step": args.step,
            "child_run_id": args.child_run_id,
            "command": args.command,
            "at": args.at or now_utc(),
        },
    )
    return 0


def finish_step(args: argparse.Namespace) -> int:
    paths = run_paths(args.output_root, args.run_id)
    state = read_state(paths)
    state.setdefault("phases", []).append(
        {
            "name": args.step,
            "status": args.status,
            "elapsed_ms": float(args.elapsed_ms),
        }
    )
    write_state(paths, state)
    append_json_line(
        paths.events_path,
        {
            "type": "step.finished",
            "run_id": args.run_id,
            "step": args.step,
            "child_run_id": args.child_run_id,
            "status": args.status,
            "exit_code": int(args.exit_code),
            "elapsed_ms": float(args.elapsed_ms),
            "at": args.at or now_utc(),
        },
    )
    return 0


def record_artifact(args: argparse.Namespace) -> int:
    paths = run_paths(args.output_root, args.run_id)
    state = read_state(paths)
    path = Path(args.path)
    entry = {
        "kind": args.kind,
        "path": args.path,
        "bytes": path.stat().st_size if path.exists() else None,
    }
    state.setdefault("artifacts", []).append(entry)
    write_state(paths, state)
    append_json_line(
        paths.events_path,
        {
            "type": "artifact.written",
            "run_id": args.run_id,
            **entry,
        },
    )
    return 0


def finish_run(args: argparse.Namespace) -> int:
    paths = run_paths(args.output_root, args.run_id)
    state = read_state(paths)
    ended_at = now_utc()

    if args.phase_name:
        state.setdefault("phases", []).append(
            {
                "name": args.phase_name,
                "status": args.phase_status,
                "elapsed_ms": float(args.phase_elapsed_ms),
            }
        )

    summary = {
        "schema_name": RUN_SCHEMA_NAME,
        "schema_version": SCHEMA_VERSION,
        "run_id": state["run_id"],
        "task_id": state.get("task_id"),
        "parent_run_id": state.get("parent_run_id"),
        "tool": state["tool"],
        "cwd": state["cwd"],
        "worktree_root": state["worktree_root"],
        "argv": state.get("argv", []),
        "target_path": state.get("target_path"),
        "command_metadata": state.get("command_metadata"),
        "status": args.status,
        "exit_code": int(args.exit_code),
        "started_at": state["started_at"],
        "ended_at": ended_at,
        "duration_ms": duration_ms(state["started_at"], ended_at),
        "phases": state.get("phases", []),
        "artifacts": {
            "bundle_dir": str(paths.bundle_dir),
            "emitted": state.get("artifacts", []),
        },
        "error": None,
        "legacy_signals": legacy_signals(),
    }
    artifacts = {
        "schema_name": ARTIFACT_SCHEMA_NAME,
        "schema_version": SCHEMA_VERSION,
        "run_id": state["run_id"],
        "items": state.get("artifacts", []),
    }
    latest = {
        "schema_name": LATEST_SCHEMA_NAME,
        "schema_version": SCHEMA_VERSION,
        "run_id": state["run_id"],
        "status": args.status,
        "summary_path": str(paths.summary_path),
        "ended_at": ended_at,
    }
    task_entry = {
        "schema_name": TASK_SCHEMA_NAME,
        "schema_version": SCHEMA_VERSION,
        "run_id": state["run_id"],
        "tool": state["tool"]["kind"],
        "command": state["tool"]["command"],
        "status": args.status,
        "started_at": state["started_at"],
        "ended_at": ended_at,
    }

    paths.summary_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    paths.artifacts_path.write_text(json.dumps(artifacts, indent=2) + "\n", encoding="utf-8")
    ensure_parent(paths.output_root / "latest.json")
    (paths.output_root / "latest.json").write_text(
        json.dumps(latest, indent=2) + "\n", encoding="utf-8"
    )
    append_json_line(
        paths.output_root / "tasks" / state["task_id"] / "runs.jsonl",
        task_entry,
    )
    append_json_line(
        paths.events_path,
        {
            "type": "run.finished",
            "run_id": state["run_id"],
            "status": args.status,
            "exit_code": int(args.exit_code),
            "ended_at": ended_at,
        },
    )
    try:
        paths.state_path.unlink()
    except FileNotFoundError:
        pass
    return 0


def parse_timestamp(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def duration_ms(started_at: str, ended_at: str) -> float:
    started = parse_timestamp(started_at)
    ended = parse_timestamp(ended_at)
    return (ended - started).total_seconds() * 1000.0


def legacy_signals() -> dict[str, Any]:
    return {
        "profile_enabled": bool(os.getenv("TONIC_PROFILE_STDERR") or os.getenv("TONIC_PROFILE_OUT")),
        "debug_cache": os.getenv("TONIC_DEBUG_CACHE") is not None,
        "debug_module_loads": os.getenv("TONIC_DEBUG_MODULE_LOADS") is not None,
        "debug_types": os.getenv("TONIC_DEBUG_TYPES") is not None,
        "memory_stats": os.getenv("TONIC_MEMORY_STATS") is not None,
        "memory_mode": os.getenv("TONIC_MEMORY_MODE") or None,
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command_name", required=True)

    start = subparsers.add_parser("start-run")
    start.add_argument("--output-root", required=True)
    start.add_argument("--tool-kind", required=True)
    start.add_argument("--tool-name", required=True)
    start.add_argument("--command", required=True)
    start.add_argument("--cwd", required=True)
    start.add_argument("--worktree-root", required=True)
    start.add_argument("--run-id")
    start.add_argument("--task-id")
    start.add_argument("--parent-run-id")
    start.add_argument("--started-at")
    start.add_argument("--argv", nargs="*")
    start.add_argument("--argv-item", action="append", default=[])
    start.set_defaults(func=start_run)

    step_start = subparsers.add_parser("start-step")
    step_start.add_argument("--output-root", required=True)
    step_start.add_argument("--run-id", required=True)
    step_start.add_argument("--step", required=True)
    step_start.add_argument("--child-run-id", required=True)
    step_start.add_argument("--command", required=True)
    step_start.add_argument("--at")
    step_start.set_defaults(func=start_step)

    step_finish = subparsers.add_parser("finish-step")
    step_finish.add_argument("--output-root", required=True)
    step_finish.add_argument("--run-id", required=True)
    step_finish.add_argument("--step", required=True)
    step_finish.add_argument("--child-run-id", required=True)
    step_finish.add_argument("--status", required=True)
    step_finish.add_argument("--exit-code", required=True)
    step_finish.add_argument("--elapsed-ms", required=True)
    step_finish.add_argument("--at")
    step_finish.set_defaults(func=finish_step)

    artifact = subparsers.add_parser("record-artifact")
    artifact.add_argument("--output-root", required=True)
    artifact.add_argument("--run-id", required=True)
    artifact.add_argument("--kind", required=True)
    artifact.add_argument("--path", required=True)
    artifact.set_defaults(func=record_artifact)

    finish = subparsers.add_parser("finish-run")
    finish.add_argument("--output-root", required=True)
    finish.add_argument("--run-id", required=True)
    finish.add_argument("--status", required=True)
    finish.add_argument("--exit-code", required=True)
    finish.add_argument("--phase-name")
    finish.add_argument("--phase-status")
    finish.add_argument("--phase-elapsed-ms")
    finish.set_defaults(func=finish_run)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
