#!/usr/bin/env python3
"""Validate checked performance evidence and ten-percent budgets."""

from __future__ import annotations

import json
import sys
from pathlib import Path


ROOT = Path(__file__).parents[2]


def validate(path: Path) -> list[str]:
    record = json.loads(path.read_text(encoding="utf-8"))
    problems: list[str] = []
    required = [
        "profile",
        "content_digest",
        "supported_files",
        "source_bytes",
        "source_reads",
        "git_processes",
        "allocation_count",
        "reallocation_count",
        "allocated_bytes",
        "samples_wall_time_ns",
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
    for observed, budget, label in [
        (record["p95_wall_time_ns"], record["wall_time_budget_ns"], "wall time"),
        (record["peak_resident_bytes"], record["peak_resident_budget_bytes"], "memory"),
    ]:
        room = budget / observed
        if room < 1.09 or room > 1.11:
            problems.append(f"{path.name}: {label} budget must have ten percent room")
    if record["source_reads"] != record["supported_files"]:
        problems.append(f"{path.name}: source reads must equal supported files")
    return problems


def main() -> int:
    paths = sorted((ROOT / "benchmarks" / "baselines").glob("*.json"))
    problems = [problem for path in paths for problem in validate(path)]
    if not paths:
        problems.append("no performance baselines found")
    if problems:
        for problem in problems:
            print(f"performance baseline: {problem}", file=sys.stderr)
        return 1
    print(f"performance baselines: {len(paths)} checked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
