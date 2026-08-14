#!/usr/bin/env python3
"""Check the public result before a generated workload is measured."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import jsonschema


DIFF_PROFILES = {"small-diff", "large-dependency-diff"}
GRAPH_PROFILES = {
    "graph-sparse",
    "graph-dense",
    "many-package",
    "evolution-dense",
    "large-dependency-diff",
}
EXPECTED_WORK = {
    "one-file": [1, 13, 1, 0, 2, 1, 3],
    "hundred-file": [1, 112, 100, 0, 2, 100, 102],
    "graph-sparse": [1, 1208, 1000, 0, 2, 1000, 1002],
    "graph-dense": [1, 608, 500, 0, 2, 500, 502],
    "many-package": [1, 3008, 1000, 0, 2, 1000, 1002],
    "evolution-dense": [1, 308, 100, 0, 3, 100, 102],
    "small-diff": [1, 112, 100, 6, 7, 104, 112],
    "large-dependency-diff": [1, 1208, 1000, 202, 7, 1200, 1404],
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


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"performance correctness: {message}")


def audit_private_values(value: object, location: str = "report") -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            require(key not in PRIVATE_KEYS, f"private field {location}.{key}")
            audit_private_values(child, f"{location}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            audit_private_values(child, f"{location}[{index}]")
    elif isinstance(value, str):
        require(not Path(value).is_absolute(), f"absolute path at {location}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--workload", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--schema", type=Path, required=True)
    parser.add_argument("--work-evidence", type=Path, required=True)
    parser.add_argument("--expected-digest")
    parser.add_argument("--digest-output", type=Path, required=True)
    args = parser.parse_args()
    workload = json.loads(args.workload.read_text())
    work_evidence = json.loads(args.work_evidence.read_text())
    report_bytes = args.report.read_bytes()
    report = json.loads(report_bytes)
    audit_private_values(report)
    schema = json.loads(args.schema.read_text())
    try:
        jsonschema.validators.validator_for(schema).check_schema(schema)
        jsonschema.validate(report, schema)
    except jsonschema.ValidationError as error:
        raise SystemExit(f"performance correctness: schema validation failed: {error.message}") from error
    digest = hashlib.sha256(report_bytes).hexdigest()
    if args.expected_digest is not None:
        require(digest == args.expected_digest, "report bytes changed; use the explicit baseline update workflow after review")

    expected_mode = "diff" if args.profile in DIFF_PROFILES else "codebase"
    for field, expected in zip(
        (
            "inventory_walks",
            "inventory_visits",
            "source_reads",
            "object_reads",
            "git_processes",
            "parser_visits",
            "algorithm_passes",
        ),
        EXPECTED_WORK[args.profile],
    ):
        require(work_evidence.get(field) == expected, f"{field} changed")
    require(report.get("schema_version") == 3, "report is not schema version 3")
    require(report.get("mode") == expected_mode, "report mode does not match workload")
    for table in (
        "paths",
        "packages",
        "scopes",
        "files",
        "findings",
        "dependency_edges",
        "package_graph",
        "file_history",
        "package_history",
        "change_coupling",
    ):
        require(isinstance(report.get(table), list), f"missing table {table}")
    require(len(report["files"]) == sum(workload["language_files"].values()), "file count changed")
    require(len(report["file_history"]) == len(report["files"]), "file history is incomplete")

    path_count = len(report["paths"])
    scope_count = len(report["scopes"])
    file_count = len(report["files"])
    package_count = len(report["packages"])
    indexed_tables = (
        "packages", "scopes", "files", "findings", "diagnostics", "comparisons",
        "health", "activity", "dependency_edges", "package_edges",
        "architecture_findings", "architecture_comparisons", "evolutionary_findings",
        "evolutionary_comparisons",
    )
    for table in indexed_tables:
        require(
            [row["id"] for row in report[table]] == list(range(len(report[table]))),
            f"{table} ids are not dense and stable",
        )
    for field in ("root", "selected_scope"):
        require(report[field] is None or report[field] < scope_count, f"{field} index is invalid")
    for package in report["packages"]:
        require(package["scope"] < scope_count, "package scope index is invalid")
        require(package["path"] == "." or not Path(package["path"]).is_absolute(), "package path is not repository-relative")
        scope = report["scopes"][package["scope"]]
        require(scope["kind"] == "package", "package scope has the wrong kind")
        require(report["paths"][scope["path"]] == package["path"], "package path and scope disagree")
    scope_links = {
        "findings": len(report["findings"]),
        "comparisons": len(report["comparisons"]),
        "architecture_findings": len(report["architecture_findings"]),
        "architecture_comparisons": len(report["architecture_comparisons"]),
        "evolutionary_findings": len(report["evolutionary_findings"]),
        "evolutionary_comparisons": len(report["evolutionary_comparisons"]),
    }
    for scope in report["scopes"]:
        require(scope["path"] is None or scope["path"] < path_count, "scope path index is invalid")
        require(scope["parent"] is None or scope["parent"] < scope_count, "scope parent index is invalid")
        require(all(child < scope_count for child in scope["children"]), "scope child index is invalid")
        require(scope["health"] < len(report["health"]), "scope health index is invalid")
        for field, limit in scope_links.items():
            require(all(index < limit for index in scope[field]), f"scope {field} index is invalid")
    for file in report["files"]:
        require(file["path"] < path_count, "file path index is invalid")
        require(file["scope"] < scope_count, "file scope index is invalid")
        require(file["package"] < package_count, "file package index is invalid")
        require(file["health"] < len(report["health"]), "file health index is invalid")
        require(file["activity"] < len(report["activity"]), "file activity index is invalid")
        expected_trust = {"parsed": "trusted", "recovered": "advisory", "failed": "failed"}
        if file["parse_outcome"] is not None:
            require(file["trust"] == expected_trust[file["parse_outcome"]], "parse outcome and trust disagree")
    for finding in report["findings"]:
        require(finding["file"] < file_count, "finding file index is invalid")
        file = report["files"][finding["file"]]
        require(finding["role"] == file["role"] and finding["trust"] == file["trust"], "finding evidence disagrees with its file")
    for diagnostic in report["diagnostics"]:
        require(diagnostic["file"] is None or diagnostic["file"] < file_count, "diagnostic file index is invalid")
    for comparison in report["comparisons"]:
        require(comparison["file"] is None or comparison["file"] < file_count, "comparison file index is invalid")
    for activity in report["activity"]:
        require(activity["file"] < file_count, "activity file index is invalid")
    for edge in report["dependency_edges"]:
        require(edge["source"] < file_count and edge["target"] < file_count, "edge index is invalid")
        source = report["files"][edge["source"]]
        require(edge["role"] == source["role"] and edge["trust"] == source["trust"], "relation evidence disagrees with its file")
    for row in report["external_dependencies"] + report["resolution_diagnostics"]:
        require(row["file"] < file_count, "relation file index is invalid")
        file = report["files"][row["file"]]
        require(row["role"] == file["role"] and row["trust"] == file["trust"], "relation context disagrees with its file")
    for edge in report["package_edges"]:
        require(edge["source"] < package_count and edge["target"] < package_count, "package edge index is invalid")
        for file_edge in edge["file_edges"]:
            require(file_edge < len(report["dependency_edges"]), "package file edge index is invalid")
            relation = report["dependency_edges"][file_edge]
            require(relation["relation"] == "uses" and relation["trust"] == "trusted", "verdict graph contains context relation")
    for finding in report["architecture_findings"]:
        require(all(package < package_count for package in finding["packages"]), "architecture package index is invalid")
        require(all(file < file_count for file in finding["files"]), "architecture file index is invalid")
        for edge in finding["witness_edges"]:
            require(edge < len(report["dependency_edges"]), "architecture witness index is invalid")
            relation = report["dependency_edges"][edge]
            require(relation["relation"] == "uses" and relation["trust"] == "trusted", "architecture witness contains context relation")
    for comparison in report["architecture_comparisons"]:
        require(all(package < package_count for package in comparison["packages"] + comparison["witness"]), "architecture comparison package index is invalid")
        require(all(file < file_count for file in comparison["files"]), "architecture comparison file index is invalid")
    for row in report["package_history"] + report["contributor_concentration"]:
        require(row["package"] < package_count, "history package index is invalid")
    for row in report["file_history"]:
        require(row["file"] < file_count, "file history index is invalid")
        file = report["files"][row["file"]]
        require(row["role"] == file["role"] and row["trust"] == file["trust"], "file history evidence disagrees with its file")
    for row in report["change_coupling"]:
        require(row["left"] < package_count and row["right"] < package_count, "coupling package index is invalid")
        require(row["left"] < row["right"], "coupling package identity is not stable")
        require(row["shared_commits"] <= row["union_commits"], "coupling commit operands are invalid")
    for row in report["evolutionary_findings"]:
        require(row["left"] < package_count and row["right"] < package_count, "evolution finding package index is invalid")
        require(row["left"] < row["right"], "evolution finding package identity is not stable")
        require(row["shared_commits"] >= 3 and row["similarity"] >= 0.2, "weak coupling entered verdict findings")
        require(row["shared_commits"] <= row["union_commits"], "evolution finding commit operands are invalid")
    for row in report["evolutionary_comparisons"]:
        require(row["left"] < package_count and row["right"] < package_count, "evolution comparison package index is invalid")
        require(row["left"] < row["right"], "evolution comparison package identity is not stable")
        require(row["shared_commits"] >= 3 and row["similarity"] >= 0.2, "weak coupling entered verdict comparisons")
        require(row["shared_commits"] <= row["union_commits"], "evolution comparison commit operands are invalid")

    if args.profile in GRAPH_PROFILES:
        require(report["dependency_edges"], "graph workload produced no internal edge")
    if args.profile == "evolution-dense":
        require(report["change_coupling"], "evolution workload produced no coupling")
    if expected_mode == "diff":
        root = report["root"]
        selected = report["scopes"][root]["coverage"]["selected_files"]
        require(selected == workload["diff_files"], "diff selection count changed")
    args.digest_output.write_text(digest + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
