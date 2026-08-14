#!/usr/bin/env python3
"""Validate privacy-safe aggregate workload review evidence."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).parents[2]
EVIDENCE = ROOT / "benchmarks" / "evidence" / "workload-reviews.json"
EXPECTED = {
    "self": {
        "important_debt_leads",
        "empty_optional_sections_absent",
    },
    "private_mixed_application": {
        "primary_application_findings_lead",
        "generated_rails_schema_outside_default_debt",
    },
    "private_rust_workspace": {
        "useful_rust_findings_lead",
        "weak_history_and_graph_facts_absent",
    },
}
PRIVATE_KEYS = {
    "source_text",
    "commit_message",
    "author",
    "authors",
    "author_name",
    "author_address",
    "author_email",
    "author_identity",
    "raw_author",
    "raw_author_name",
    "raw_author_address",
    "raw_author_email",
    "address",
    "email",
    "email_address",
    "contributor_id",
    "contributor_ids",
    "contributor_identity",
    "contributor_identities",
    "contributor_name",
    "contributor_address",
    "contributor_email",
}


def audit(value: object, location: str = "evidence") -> list[str]:
    problems: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            if key in PRIVATE_KEYS:
                problems.append(f"private field {location}.{key}")
            problems.extend(audit(child, f"{location}.{key}"))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            problems.extend(audit(child, f"{location}[{index}]"))
    elif isinstance(value, str) and Path(value).is_absolute():
        problems.append(f"absolute path at {location}")
    return problems


def validate(record: dict, baselines: list[dict], release_head: bool, head: str | None = None) -> list[str]:
    problems = audit(record)
    if set(record) != {"schema_version", "workspace_revision", "workspace_dirty", "reviews"}:
        problems.append("top-level fields changed")
    if record.get("schema_version") != 1:
        problems.append("schema version changed")
    if record.get("workspace_dirty") is not False:
        problems.append("review must start from a clean workspace")
    revision = record.get("workspace_revision")
    if not isinstance(revision, str) or len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision):
        problems.append("review revision must be a full Git object id")
    reviews = record.get("reviews", [])
    if not isinstance(reviews, list) or [row.get("family") for row in reviews] != list(EXPECTED):
        problems.append("workload families changed")
    else:
        for row in reviews:
            if set(row) != {"family", "outcomes"}:
                problems.append(f"{row.get('family')}: review fields changed")
                continue
            expected = EXPECTED[row["family"]]
            outcomes = row["outcomes"]
            if not isinstance(outcomes, dict) or set(outcomes) != expected:
                problems.append(f"{row['family']}: outcome categories changed")
            elif any(value is not True for value in outcomes.values()):
                problems.append(f"{row['family']}: expected outcome failed")

    baseline_revisions = {row.get("workspace_revision") for row in baselines}
    if len(baselines) != 8:
        problems.append("exactly eight public profiles are required")
    if baseline_revisions != {revision}:
        problems.append("workload reviews and public profiles must share one revision")
    if any(row.get("workspace_dirty") is not False for row in baselines):
        problems.append("public profiles must start from a clean workspace")
    if release_head:
        if head is None:
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
            ).stdout.strip()
        if revision != head:
            problems.append("workload review revision must equal HEAD")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-head", action="store_true")
    parser.add_argument("--evidence", type=Path, default=EVIDENCE)
    args = parser.parse_args()
    record = json.loads(args.evidence.read_text(encoding="utf-8"))
    baselines = [
        json.loads(path.read_text())
        for path in sorted((ROOT / "benchmarks" / "baselines").glob("*.json"))
    ]
    problems = validate(record, baselines, args.release_head)
    if problems:
        for problem in problems:
            print(f"workload review: {problem}", file=sys.stderr)
        return 1
    print("workload reviews: 3 checked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
