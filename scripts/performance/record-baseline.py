#!/usr/bin/env python3
"""Combine checked workload metadata and measured runs into one baseline."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    metadata = json.loads((args.input / "metadata.json").read_text())
    runs = [json.loads(line) for line in (args.input / "runs.jsonl").read_text().splitlines()]
    if len(runs) != 5:
        raise SystemExit("baseline requires five measured runs")
    stable_fields = ("source_reads", "git_processes", "inventory_walks")
    for field in stable_fields:
        if len({run[field] for run in runs}) != 1:
            raise SystemExit(f"{field} changed between runs")
    wall_times = [run["wall_time_ns"] for run in runs]
    parser_times = [run["parser_time_ns"] for run in runs]
    peak = max(run["peak_resident_bytes"] for run in runs)
    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
    dirty = bool(
        subprocess.run(
            ["git", "status", "--porcelain"], check=True, capture_output=True, text=True
        ).stdout
    )
    record = {
        "schema_version": 1,
        "profile": args.profile,
        "source_engine": "owned-tree-sitter-static-graph",
        "workspace_revision": revision,
        "workspace_dirty": dirty,
        "host": metadata["host"],
        "rustc": metadata["rustc"],
        "command": f"scripts/performance/baseline.sh {args.profile}",
        "seed": metadata["seed"],
        "content_digest": metadata["content_digest"],
        "supported_files": sum(metadata["language_files"].values()),
        "source_bytes": metadata["source_bytes"],
        "source_reads": runs[0]["source_reads"],
        "git_processes": runs[0]["git_processes"],
        "inventory_walks": runs[0]["inventory_walks"],
        "samples_parser_time_ns": parser_times,
        "p95_parser_time_ns": max(parser_times),
        "samples_wall_time_ns": wall_times,
        "p95_wall_time_ns": max(wall_times),
        "peak_resident_bytes": peak,
        "wall_time_budget_ns": (max(wall_times) * 110 + 99) // 100,
        "peak_resident_budget_bytes": (peak * 110 + 99) // 100,
        "budget_room_percent": 10,
        "allocation_count": max(run["allocation_count"] for run in runs),
        "reallocation_count": max(run["reallocation_count"] for run in runs),
        "allocated_bytes": max(run["allocated_bytes"] for run in runs),
        "allocation_note": "Measured with the allocation-stats release feature.",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
