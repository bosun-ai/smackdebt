#!/usr/bin/env python3
"""Review real workloads without retaining their reports."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

import jsonschema


ROOT = Path(__file__).parents[2]
FAMILIES = ("self", "private_mixed_application", "private_rust_workspace")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"workload review: {message}")


def run(binary: Path, repository: Path, *arguments: str) -> str:
    result = subprocess.run(
        [str(binary), *arguments, "."],
        cwd=repository,
        check=False,
        capture_output=True,
        text=True,
    )
    require(result.returncode == 0, "analysis did not complete")
    require(not result.stderr, "analysis wrote to stderr")
    return result.stdout


def reviewed_report(binary: Path, repository: Path, schema: dict) -> tuple[dict, str]:
    report = json.loads(run(binary, repository, "--json", "--history", "36500d"))
    jsonschema.validate(report, schema)
    terminal = run(binary, repository, "--history", "36500d")
    return report, terminal


def package_references_are_valid(report: dict) -> bool:
    package_count = len(report["packages"])
    file_count = len(report["files"])
    edge_count = len(report["dependency_edges"])

    if [row["id"] for row in report["packages"]] != list(range(package_count)):
        return False
    if any(row["package"] >= package_count for row in report["files"]):
        return False
    if any(row["package"] >= package_count for row in report["package_graph"]):
        return False
    if any(
        row["source"] >= file_count or row["target"] >= file_count
        for row in report["dependency_edges"]
    ):
        return False
    if any(
        row["source"] >= package_count
        or row["target"] >= package_count
        or any(edge >= edge_count for edge in row["file_edges"])
        for row in report["package_edges"]
    ):
        return False
    if any(
        any(package >= package_count for package in row["packages"])
        or any(file >= file_count for file in row["files"])
        or any(edge >= edge_count for edge in row["witness_edges"])
        for row in report["architecture_findings"]
    ):
        return False
    if any(
        any(package >= package_count for package in row["packages"] + row["witness"])
        or any(file >= file_count for file in row["files"])
        for row in report["architecture_comparisons"]
    ):
        return False
    if any(
        row["package"] >= package_count
        for table in ("package_history", "contributor_concentration")
        for row in report[table]
    ):
        return False
    return not any(
        row["left"] >= package_count
        or row["right"] >= package_count
        or row["left"] >= row["right"]
        for table in ("change_coupling", "evolutionary_findings", "evolutionary_comparisons")
        for row in report[table]
    )


def default_coupling_meets_threshold(report: dict) -> bool:
    return all(
        row["left"] < row["right"]
        and row["shared_commits"] >= 3
        and row["similarity"] >= 0.2
        for row in report["evolutionary_findings"]
    )


def cycles_exclude_role(report: dict, role: str) -> bool:
    return all(
        all(report["files"][file]["role"] != role for file in finding["files"])
        for finding in report["architecture_findings"]
    )


def ownership_relations_do_not_form_cycles(report: dict) -> bool:
    return all(
        all(
            report["dependency_edges"][edge]["relation"] != "module_ownership"
            for edge in finding["witness_edges"]
        )
        for finding in report["architecture_findings"]
    )


def finding_path(report: dict, finding: dict) -> str:
    file = report["files"][finding["file"]]
    return report["paths"][file["path"]]


def generated_schema_client_findings_are_excluded(report: dict, terminal: str) -> bool:
    generated_schema = [
        finding
        for finding in report["findings"]
        if finding["role"] == "generated"
        and "schema" in finding_path(report, finding).lower()
    ]
    generated_client = [
        finding
        for finding in report["findings"]
        if finding["role"] == "generated"
        and "client" in finding_path(report, finding).lower()
    ]
    return bool(generated_schema) and bool(generated_client) and all(
        finding_path(report, finding) not in terminal
        for finding in generated_schema + generated_client
    )


def substantial_primary_finding_is_prominent(report: dict, terminal: str) -> bool:
    return any(
        finding["role"] == "primary"
        and finding["trust"] == "trusted"
        and finding["rating"] == "high"
        and finding["measurements"]["cognitive_complexity"] >= 25
        and finding_path(report, finding) in terminal
        for finding in report["findings"]
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--self", dest="self_repository", type=Path, required=True)
    parser.add_argument("--mixed", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    binary = args.binary.resolve()
    schema = json.loads((ROOT / "schemas" / "report-v3.schema.json").read_text())
    self_report, self_terminal = reviewed_report(binary, args.self_repository, schema)
    mixed_report, mixed_terminal = reviewed_report(binary, args.mixed, schema)
    rust_report, rust_terminal = reviewed_report(binary, args.rust, schema)

    outcomes = {
        "self": {
            "fixture_cycles_absent": cycles_exclude_role(self_report, "fixture"),
            "package_references_valid": package_references_are_valid(self_report),
            "root_label_readable": self_terminal.splitlines()[1:2] == ["repository root"],
            "default_coupling_threshold_met": default_coupling_meets_threshold(self_report),
        },
        "private_mixed_application": {
            "generated_schema_client_findings_excluded": generated_schema_client_findings_are_excluded(
                mixed_report, mixed_terminal
            ),
            "weak_coupling_absent": default_coupling_meets_threshold(mixed_report),
            "ownership_cycles_absent": ownership_relations_do_not_form_cycles(mixed_report),
            "hand_written_high_findings_visible": substantial_primary_finding_is_prominent(
                mixed_report, mixed_terminal
            ),
            "default_coupling_threshold_met": default_coupling_meets_threshold(mixed_report),
        },
        "private_rust_workspace": {
            "ownership_cycles_absent": ownership_relations_do_not_form_cycles(rust_report),
            "high_complexity_findings_visible": substantial_primary_finding_is_prominent(
                rust_report, rust_terminal
            ),
            "default_coupling_threshold_met": default_coupling_meets_threshold(rust_report),
        },
    }
    require(tuple(outcomes) == FAMILIES, "workload families changed")
    failed = [
        f"{family}.{category}"
        for family, result in outcomes.items()
        for category, passed in result.items()
        if not passed
    ]
    require(not failed, f"expected outcome failed: {', '.join(failed)}")

    evidence = {
        "schema_version": 1,
        "workspace_revision": args.revision,
        "workspace_dirty": False,
        "reviews": [
            {"family": family, "outcomes": outcomes[family]} for family in FAMILIES
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
