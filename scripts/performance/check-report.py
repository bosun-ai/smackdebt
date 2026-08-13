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


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"performance correctness: {message}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--workload", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--schema", type=Path, required=True)
    parser.add_argument("--expected-digest")
    parser.add_argument("--digest-output", type=Path, required=True)
    args = parser.parse_args()
    workload = json.loads(args.workload.read_text())
    report_bytes = args.report.read_bytes()
    report = json.loads(report_bytes)
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
    require(report.get("schema_version") == 2, "report is not schema version 2")
    require(report.get("mode") == expected_mode, "report mode does not match workload")
    for table in (
        "paths",
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
    package_count = len(report["package_graph"])
    for file in report["files"]:
        require(file["path"] < path_count, "file path index is invalid")
        require(file["scope"] < scope_count, "file scope index is invalid")
        if file["package"] is not None:
            require(file["package"] < package_count, "file package index is invalid")
    for edge in report["dependency_edges"]:
        require(edge["source"] < file_count and edge["target"] < file_count, "edge index is invalid")

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
