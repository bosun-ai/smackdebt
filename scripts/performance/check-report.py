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
    "evolution-wide",
    "large-dependency-diff",
}
# What each history workload's deliberate shape has to produce. The dense
# profile is the bulk-guard proof: both of its commits touch a hundred files, so
# both are declined and no pair survives. The wide profile is the pair proof:
# one leaky interface, four pairs no connection path joins, twelve retained
# pairs one commit below the support floor, and one sweeping commit beside the
# initial import that the guard declines.
EXPECTED_HISTORY = {
    "evolution-dense": {"bulk_commits": 2, "pairs": 0, "leaky": 0, "hidden": 0},
    "evolution-wide": {"bulk_commits": 2, "pairs": 17, "leaky": 1, "hidden": 4},
}
LEAKAGE_MIN_DISTANCE = 2
LEAKAGE_SHARED_COMMITS = 5
LEAKAGE_SIMILARITY_PERMILLE = 400
LEAKAGE_SIMILARITY_STEP_PERMILLE = 50
LEAKAGE_SIMILARITY_FLOOR_PERMILLE = 200
# Inventory visits count the runner's own bookkeeping files as well as the
# workload's source. A workload that owns a Git repository writes a
# `.git/info/exclude` naming four of them, and the walk has honored that file
# since discovery moved onto the ignore crate, so every Git-bearing profile
# visits four fewer entries than an equally sized profile without a repository.
EXPECTED_WORK = {
    "one-file": [1, 13, 1, 0, 2, 1, 3],
    "hundred-file": [1, 112, 100, 0, 2, 100, 102],
    "graph-sparse": [1, 1208, 1000, 0, 2, 1000, 1002],
    "graph-dense": [1, 608, 500, 0, 2, 500, 502],
    "many-package": [1, 3008, 1000, 0, 2, 1000, 1002],
    "evolution-dense": [1, 304, 100, 0, 3, 100, 102],
    "evolution-wide": [1, 2104, 2000, 0, 3, 2000, 2002],
    "small-diff": [1, 108, 100, 6, 7, 104, 112],
    "large-dependency-diff": [1, 1204, 1000, 202, 7, 1200, 1404],
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


def check_file_pairs(pairs: list, file_count: int) -> None:
    """Mirror the retained file pair contract: bounds, order, and floors."""
    previous = None
    for row in pairs:
        require(row["left"] < file_count and row["right"] < file_count, "file pair index is invalid")
        require(row["left"] < row["right"], "file pair identity is not stable")
        pair = (row["left"], row["right"])
        require(previous is None or previous < pair, "file pairs are not in file order")
        previous = pair
        require(row["shared_commits"] <= row["union_commits"], "file pair operands are invalid")
        require(row["shared_commits"] >= 3, "a pair below the support floor was retained")
        require(row["shared_commits"] * 10 >= row["union_commits"], "a pair below the similarity floor was retained")
        require(row["distance"] >= 1, "a same-directory file pair was retained")


def required_permille(distance: int) -> int:
    """The share of its union a pair that many directories apart must reach."""
    steps = max(0, distance - LEAKAGE_MIN_DISTANCE)
    bar = LEAKAGE_SIMILARITY_PERMILLE - steps * LEAKAGE_SIMILARITY_STEP_PERMILLE
    return max(bar, LEAKAGE_SIMILARITY_FLOOR_PERMILLE)


def check_leakage_findings(findings: list, pairs: list, file_count: int) -> None:
    """Mirror the detector floors: no finding may sit below one of them."""
    for row in findings:
        require(row["coupling"] < len(pairs), "leakage finding pair index is invalid")
        pair = pairs[row["coupling"]]
        require(pair["distance"] >= LEAKAGE_MIN_DISTANCE, "a finding below the distance floor")
        require(pair["shared_commits"] >= LEAKAGE_SHARED_COMMITS, "a finding below the support floor")
        require(
            pair["shared_commits"] * 1000 >= pair["union_commits"] * required_permille(pair["distance"]),
            "a finding below the similarity bar its distance sets",
        )
        interface = row.get("interface")
        if row["kind"] == "leaky_interface":
            require(interface in (pair["left"], pair["right"]), "a leaky interface outside its own pair")
        else:
            require(row["kind"] == "hidden_coupling", "unknown leakage finding kind")
            require(interface is None, "a hidden pair named an interface")
        require(interface is None or interface < file_count, "leakage interface index is invalid")


def check_history_tables(profile: str, report: dict, package_count: int, file_count: int) -> None:
    """Check every table history fills, from package pairs down to findings."""
    for row in report["change_coupling"]:
        require(row["left"] < package_count and row["right"] < package_count, "coupling package index is invalid")
        require(row["left"] < row["right"], "coupling package identity is not stable")
        require(row["shared_commits"] <= row["union_commits"], "coupling commit operands are invalid")
    check_file_pairs(report["file_change_coupling"], file_count)
    check_leakage_findings(report["change_leakage_findings"], report["file_change_coupling"], file_count)
    check_history_shape(profile, report)
    # Similarity is published as its integer operands rather than a ratio, so
    # the twenty-percent floor is read the way the report states it.
    for table in ("evolutionary_findings", "evolutionary_comparisons"):
        for row in report[table]:
            require(row["left"] < package_count and row["right"] < package_count, f"{table} package index is invalid")
            require(row["left"] < row["right"], f"{table} package identity is not stable")
            require(row["shared_commits"] >= 3 and row["shared_commits"] * 5 >= row["union_commits"], f"weak coupling entered {table}")
            require(row["shared_commits"] <= row["union_commits"], f"{table} commit operands are invalid")


def check_history_shape(profile: str, report: dict) -> None:
    """Prove the deliberate history of a workload reached the report intact."""
    expected = EXPECTED_HISTORY.get(profile)
    if expected is None:
        return
    coverage = report["history_coverage"]
    findings = report["change_leakage_findings"]
    observed = {
        "bulk_commits": coverage["bulk_commits"],
        "pairs": len(report["file_change_coupling"]),
        "leaky": sum(row["kind"] == "leaky_interface" for row in findings),
        "hidden": sum(row["kind"] == "hidden_coupling" for row in findings),
    }
    require(observed == expected, f"history shape changed: {observed} is not {expected}")
    require(coverage["declined_pairs"] == 0, "the pair storage limit declined a key")


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
    require(report.get("schema_version") == 4, "report is not schema version 4")
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
        "file_change_coupling",
        "change_leakage_findings",
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
    check_history_tables(args.profile, report, package_count, file_count)

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
