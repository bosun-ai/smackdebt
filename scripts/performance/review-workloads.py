#!/usr/bin/env python3
"""Review real workloads without retaining their reports."""

from __future__ import annotations

import argparse
import json
import subprocess
import unicodedata
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


def finding_path(report: dict, finding: dict) -> str:
    file = report["files"][finding["file"]]
    return report["paths"][file["path"]]


def section_lines(terminal: str, heading: str) -> list[str] | None:
    marker = f"\n{heading}\n"
    if marker not in terminal:
        return None
    remainder = terminal.split(marker, 1)[1]
    headings = {"QUALITY", "AREAS", "FINDINGS", "ARCHITECTURE", "HISTORY", "WARNINGS"}
    lines = []
    for line in remainder.splitlines():
        if line in headings or line.startswith(" "):
            break
        if not line and lines:
            continue
        lines.append(line)
    return [line for line in lines if line]


def terminal_line(value: str, width: int = 100) -> str:
    def cell_width(character: str) -> int:
        if unicodedata.combining(character):
            return 0
        return 2 if unicodedata.east_asian_width(character) in {"W", "F"} else 1

    if sum(cell_width(character) for character in value) <= width:
        return value
    retained = []
    used = 0
    for character in value:
        character_width = cell_width(character)
        if used + character_width > width - 1:
            break
        retained.append(character)
        used += character_width
    return "".join(retained) + "…"


def first_displayed_finding(report: dict, terminal: str) -> dict | None:
    lines = section_lines(terminal, "FINDINGS")
    if not lines:
        return None
    for index, line in enumerate(lines):
        if not line.startswith((" ", " ")):
            continue
        name = line[1:].strip().split(" · ", 1)[0]
        path_line = lines[index + 1].strip() if index + 1 < len(lines) else ""
        for finding in report["findings"]:
            path = finding_path(report, finding)
            container = finding.get("container")
            identity = f"{container}::{finding['name']}" if container else finding["name"]
            location = f"{path}:{finding['start_line']}"
            if identity == name and path_line == location:
                return finding
        return None
    return None


def substantial_trusted_finding(finding: dict | None) -> bool:
    return bool(
        finding
        and finding["trust"] == "trusted"
        and finding["rating"] == "high"
        and finding["measurements"]["cognitive_complexity"] >= 25
    )


def finding_section_leads(terminal: str) -> bool:
    finding = terminal.find("\nFINDINGS\n")
    if finding < 0:
        return False
    later_sections = [
        position
        for heading in ("\nARCHITECTURE\n", "\nHISTORY\n", "\nWARNINGS\n")
        if (position := terminal.find(heading)) >= 0
    ]
    return not later_sections or finding < min(later_sections)


def important_debt_leads(report: dict, terminal: str) -> bool:
    finding = first_displayed_finding(report, terminal)
    return substantial_trusted_finding(finding) and finding_section_leads(terminal)


def primary_application_finding_leads(report: dict, terminal: str) -> bool:
    finding = first_displayed_finding(report, terminal)
    return bool(
        substantial_trusted_finding(finding)
        and finding["role"] == "primary"
        and finding_section_leads(terminal)
    )


def useful_rust_finding_leads(report: dict, terminal: str) -> bool:
    finding = first_displayed_finding(report, terminal)
    return bool(
        substantial_trusted_finding(finding)
        and finding_path(report, finding).endswith(".rs")
        and finding_section_leads(terminal)
    )


def empty_optional_sections_are_absent(report: dict, terminal: str) -> bool:
    root = report["scopes"][report["root"]]
    affected_children = [
        report["scopes"][child]
        for child in root["children"]
        if report["health"][report["scopes"][child]["health"]]["high"]
        + report["health"][report["scopes"][child]["health"]]["watch"]
        > 0
    ]
    return (
        (len(affected_children) >= 2 or "\nAREAS\n" not in terminal)
        and (bool(root["architecture_findings"]) or "\nARCHITECTURE\n" not in terminal)
        and (bool(root["evolutionary_findings"]) or "\nHISTORY\n" not in terminal)
    )


def generated_rails_schema_is_outside_default_debt(report: dict, terminal: str) -> bool:
    findings = [
        finding
        for finding in report["findings"]
        if finding["role"] == "generated"
        and finding_path(report, finding).lower().endswith("db/schema.rb")
    ]
    return bool(findings) and all(finding_path(report, finding) not in terminal for finding in findings)


def displayed_package_name(package: dict) -> str:
    path = package["path"]
    return "repository root" if path == "." else path


def strongest_history_rows(report: dict, finding_ids: list[int]) -> list[str] | None:
    packages = report["packages"]
    findings = report["evolutionary_findings"]
    selected = []
    for finding_id in finding_ids:
        if not isinstance(finding_id, int) or not 0 <= finding_id < len(findings):
            return None
        finding = findings[finding_id]
        left = finding.get("left")
        right = finding.get("right")
        shared = finding.get("shared_commits")
        union = finding.get("union_commits")
        similarity = finding.get("similarity")
        if (
            not isinstance(left, int)
            or not isinstance(right, int)
            or not 0 <= left < len(packages)
            or not 0 <= right < len(packages)
            or not isinstance(shared, int)
            or not isinstance(union, int)
            or not isinstance(similarity, (int, float))
            or shared < 3
            or union < shared
            or shared * 5 < union
        ):
            return None
        selected.append(finding)

    selected.sort(
        key=lambda finding: (
            -finding["shared_commits"],
            -finding["similarity"],
            displayed_package_name(packages[finding["left"]]),
            displayed_package_name(packages[finding["right"]]),
            finding["left"],
            finding["right"],
        )
    )
    rows = []
    package_edges = report.get("package_edges", [])
    for finding in selected[:3]:
        left = finding["left"]
        right = finding["right"]
        dependency = any(
            {edge.get("source"), edge.get("target")} == {left, right}
            for edge in package_edges
        )
        percentage = int(finding["similarity"] * 100 + 0.5)
        rows.append(
            terminal_line(
                f" {displayed_package_name(packages[left])} ↔ "
                f"{displayed_package_name(packages[right])} "
                f"changed together in {finding['shared_commits']} of "
                f"{finding['union_commits']} commits · {percentage}% · "
                f"{'code dependency exists' if dependency else 'no code dependency'}"
            )
        )
    return rows


def expected_architecture_rows(report: dict, finding_ids: list[int]) -> list[str] | None:
    findings = report.get("architecture_findings", [])
    edges = report.get("dependency_edges", [])
    files = report.get("files", [])
    paths = report.get("paths", [])
    rows = []
    labels = {
        "package_cycle": ("", "package dependency cycle"),
        "file_cycle": ("", "file dependency cycle"),
    }
    for finding_id in finding_ids[:3]:
        if not isinstance(finding_id, int) or not 0 <= finding_id < len(findings):
            return None
        finding = findings[finding_id]
        if finding.get("kind") not in labels:
            return None
        glyph, label = labels[finding["kind"]]
        rows.append(terminal_line(f"{glyph} {label}"))
        witness = finding.get("witness_edges", [])
        if not witness:
            continue
        witness_edges = []
        for edge_id in witness:
            if not isinstance(edge_id, int) or not 0 <= edge_id < len(edges):
                return None
            edge = edges[edge_id]
            source = edge.get("source")
            target = edge.get("target")
            if (
                not isinstance(source, int)
                or not isinstance(target, int)
                or not 0 <= source < len(files)
                or not 0 <= target < len(files)
            ):
                return None
            witness_edges.append(edge)
        first = witness_edges[0]
        file_ids = [first["source"], first["target"]]
        file_ids.extend(edge["target"] for edge in witness_edges[1:])
        try:
            witness_paths = [paths[files[file_id]["path"]] for file_id in file_ids]
        except (IndexError, KeyError, TypeError):
            return None
        rows.append(terminal_line("        " + " → ".join(witness_paths)))
    return rows


def weak_history_and_graph_facts_are_absent(report: dict, terminal: str) -> bool:
    root = report["scopes"][report["root"]]
    expected_history = strongest_history_rows(report, root["evolutionary_findings"])
    if expected_history is None:
        return False
    history = section_lines(terminal, "HISTORY")
    if expected_history:
        if history != expected_history:
            return False
    elif history is not None:
        return False

    expected_architecture = expected_architecture_rows(
        report, root["architecture_findings"]
    )
    if expected_architecture is None:
        return False
    architecture = section_lines(terminal, "ARCHITECTURE")
    if expected_architecture:
        return architecture == expected_architecture
    return architecture is None


def review_outcomes(
    self_report: dict,
    self_terminal: str,
    mixed_report: dict,
    mixed_terminal: str,
    rust_report: dict,
    rust_terminal: str,
) -> dict[str, dict[str, bool]]:
    return {
        "self": {
            "important_debt_leads": important_debt_leads(self_report, self_terminal),
            "empty_optional_sections_absent": empty_optional_sections_are_absent(
                self_report, self_terminal
            ),
        },
        "private_mixed_application": {
            "primary_application_findings_lead": primary_application_finding_leads(
                mixed_report, mixed_terminal
            ),
            "generated_rails_schema_outside_default_debt": generated_rails_schema_is_outside_default_debt(
                mixed_report, mixed_terminal
            ),
        },
        "private_rust_workspace": {
            "useful_rust_findings_lead": useful_rust_finding_leads(
                rust_report, rust_terminal
            ),
            "weak_history_and_graph_facts_absent": weak_history_and_graph_facts_are_absent(
                rust_report, rust_terminal
            ),
        },
    }


def non_boolean_outcomes(outcomes: dict) -> list[str]:
    return [
        f"{family}.{category}"
        for family, result in outcomes.items()
        for category, value in result.items()
        if type(value) is not bool
    ]


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

    outcomes = review_outcomes(
        self_report,
        self_terminal,
        mixed_report,
        mixed_terminal,
        rust_report,
        rust_terminal,
    )
    require(tuple(outcomes) == FAMILIES, "workload families changed")
    invalid_types = non_boolean_outcomes(outcomes)
    require(
        not invalid_types,
        f"outcomes must be JSON booleans: {', '.join(invalid_types)}",
    )
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
