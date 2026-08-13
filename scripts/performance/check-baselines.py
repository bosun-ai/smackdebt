#!/usr/bin/env python3
"""Validate checked performance evidence and ten-percent budgets."""

from __future__ import annotations

import json
import subprocess
import sys
import argparse
from pathlib import Path


ROOT = Path(__file__).parents[2]


def validate(path: Path) -> list[str]:
    record = json.loads(path.read_text(encoding="utf-8"))
    problems: list[str] = []
    required = [
        "profile",
        "content_digest",
        "report_digest",
        "supported_files",
        "source_bytes",
        "source_reads",
        "object_reads",
        "git_processes",
        "inventory_visits",
        "parser_visits",
        "algorithm_passes",
        "allocation_count",
        "reallocation_count",
        "allocated_bytes",
        "workspace_revision",
        "workspace_dirty",
        "host",
        "rustc",
        "samples_wall_time_ns",
        "samples_parser_time_ns",
        "p95_parser_time_ns",
        "p95_wall_time_ns",
        "peak_resident_bytes",
        "wall_time_budget_ns",
        "peak_resident_budget_bytes",
    ]
    for field in required:
        if field not in record:
            problems.append(f"{path.name}: missing {field}")
    if problems:
        return problems
    if max(record["samples_wall_time_ns"]) != record["p95_wall_time_ns"]:
        problems.append(f"{path.name}: five-sample p95 must equal the slowest sample")
    if max(record["samples_parser_time_ns"]) != record["p95_parser_time_ns"]:
        problems.append(f"{path.name}: parser p95 must equal the slowest sample")
    if record["p95_parser_time_ns"] <= 0:
        problems.append(f"{path.name}: parser time must be observable")
    if len(record["report_digest"]) != 64 or any(
        value not in "0123456789abcdef" for value in record["report_digest"]
    ):
        problems.append(f"{path.name}: report digest must be lowercase SHA-256")
    for observed, budget, label in [
        (record["p95_wall_time_ns"], record["wall_time_budget_ns"], "wall time"),
        (record["peak_resident_bytes"], record["peak_resident_budget_bytes"], "memory"),
    ]:
        room = budget / observed
        if room < 1.09 or room > 1.11:
            problems.append(f"{path.name}: {label} budget must have ten percent room")
    if record["source_reads"] != record["supported_files"]:
        problems.append(f"{path.name}: source reads must equal supported files")
    if record["parser_visits"] < record["source_reads"]:
        problems.append(f"{path.name}: every source read must reach a parser visit")
    if not isinstance(record["workspace_dirty"], bool):
        problems.append(f"{path.name}: workspace dirty state must be recorded")
    for field in ("workspace_revision", "host", "rustc"):
        if not record[field]:
            problems.append(f"{path.name}: {field} must be recorded")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-head", action="store_true")
    args = parser.parse_args()
    paths = sorted((ROOT / "benchmarks" / "baselines").glob("*.json"))
    problems = [problem for path in paths for problem in validate(path)]
    if not paths:
        problems.append("no performance baselines found")
    records = [json.loads(path.read_text()) for path in paths]
    revisions = {record.get("workspace_revision") for record in records}
    if len(revisions) != 1:
        problems.append("performance baselines must share one workspace revision")
    if args.release_head:
        head = subprocess.run(
            ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
        ).stdout.strip()
        if any(record.get("workspace_dirty") is not False for record in records):
            problems.append("release baselines must start from a clean workspace")
        if revisions != {head}:
            problems.append("release baseline revision must equal HEAD")
    if problems:
        for problem in problems:
            print(f"performance baseline: {problem}", file=sys.stderr)
        return 1
    print(f"performance baselines: {len(paths)} checked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
