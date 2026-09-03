#!/usr/bin/env python3
"""Review real workloads without retaining their reports."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path

import jsonschema

from workload_review_warnings import warnings_match_report


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


CODEBASE_HEADINGS = {"AREAS", "PROBLEMS", "WARNINGS"}
RETIRED_CODEBASE_HEADINGS = {"QUALITY", "FINDINGS", "ARCHITECTURE", "HISTORY"}
CODEBASE_TIER_SENTENCES = {
    "empty": "Nothing was checked.",
    "clean": "Clean. Ship it.",
    "solid": "Solid, with rough edges.",
    "worn": "Worn in the usual places.",
    "fights_back": "This code fights back.",
    "lost": "The code is winning.",
}


def section_lines(terminal: str, heading: str) -> list[str] | None:
    marker = f"\n{heading}\n"
    if marker not in terminal:
        return None
    remainder = terminal.split(marker, 1)[1]
    headings = CODEBASE_HEADINGS | RETIRED_CODEBASE_HEADINGS
    lines = []
    for line in remainder.splitlines():
        if line in headings or line.lstrip().startswith("next: "):
            break
        if not line and lines:
            continue
        lines.append(line)
    return [line for line in lines if line]


def default_problems(report: dict) -> list[dict]:
    return [
        problem
        for problem in report.get("problems", [])
        if problem.get("visibility") == "default"
    ]


CLAIM_TABLES = (
    "findings",
    "size_findings",
    "architecture_findings",
    "stable_dependency_findings",
    "evolutionary_findings",
    "knowledge_concentration_findings",
    "change_leakage_findings",
)


def problem_references_are_valid(report: dict) -> bool:
    problems = report.get("problems", [])
    evidence = [problem_evidence_references(problem) for problem in problems]
    claimed = [problem_claim_references(problem) for problem in problems]
    valid_evidence = all(
        table in CLAIM_TABLES and valid_index(index, report[table])
        for references in evidence
        for table, index in references
    )
    claims_have_evidence = all(
        set(claims) <= facts for claims, facts in zip(claimed, evidence)
    )
    expected = [
        (table, index)
        for table in CLAIM_TABLES
        for index in range(len(report.get(table, [])))
    ]
    return valid_evidence and claims_have_evidence and sorted(sum(claimed, [])) == sorted(expected)


def problem_evidence_references(problem: dict) -> set[tuple[object, object]]:
    return {
        (row.get("kind"), row.get("index"))
        for row in problem.get("evidence", [])
        if "index" in row
    }


def problem_claim_references(problem: dict) -> list[tuple[object, object]]:
    return [
        (claim.get("table"), claim.get("index"))
        for claim in problem.get("claimed", [])
    ]


def valid_index(value, rows: list) -> bool:
    return isinstance(value, int) and 0 <= value < len(rows)


def file_path(report: dict, file) -> str | None:
    if not valid_index(file, report["files"]):
        return None
    return report["paths"][report["files"][file]["path"]]


def file_problem_anchor(report: dict, problem: dict) -> str | None:
    path = file_path(report, problem["anchor"].get("file"))
    evidence = problem.get("evidence", [])
    if path and problem.get("pattern") == "measured" and evidence:
        first = evidence[0]
        if first.get("kind") == "findings":
            index = first.get("index")
            if not valid_index(index, report["findings"]):
                return None
            finding = report["findings"][index]
            return f"{path}:{finding['start_line']}"
    return path


def package_problem_anchor(report: dict, problem: dict) -> str | None:
    package = problem["anchor"].get("package")
    if not valid_index(package, report["packages"]):
        return None
    return displayed_package_name(report["packages"][package])


def package_pair_problem_anchor(report: dict, problem: dict) -> str | None:
    anchor = problem["anchor"]
    indexes = tuple(anchor.get("packages", []))
    if len(indexes) != 2:
        return None
    if not all(valid_index(index, report["packages"]) for index in indexes):
        return None
    left, right = (displayed_package_name(report["packages"][index]) for index in indexes)
    arrow = "→" if problem.get("pattern") == "unstable_dependency" else "↔"
    return f"{left} {arrow} {right}"


def cycle_witness_anchor(report: dict, problem: dict) -> str | None:
    for evidence in problem.get("evidence", []):
        if evidence.get("kind") != "architecture_findings":
            continue
        index = evidence.get("index")
        if not valid_index(index, report["architecture_findings"]):
            return None
        finding = report["architecture_findings"][index]
        witness = finding.get("witness_edges", [])
        if not witness:
            continue
        edge_index = witness[0]
        if not valid_index(edge_index, report["dependency_edges"]):
            return None
        return file_path(report, report["dependency_edges"][edge_index].get("source"))
    return None


def files_problem_anchor(report: dict, problem: dict) -> str | None:
    paths = [file_path(report, file) for file in problem["anchor"].get("files", [])]
    if not paths or any(path is None for path in paths):
        return None
    if problem.get("pattern") == "hidden_coupling":
        return " ↔ ".join(paths[:2])
    return cycle_witness_anchor(report, problem) or min(paths)


def problem_anchor(report: dict, problem: dict) -> str | None:
    parser = {
        "file": file_problem_anchor,
        "files": files_problem_anchor,
        "package": package_problem_anchor,
        "package_pair": package_pair_problem_anchor,
    }.get(problem.get("anchor", {}).get("kind"))
    return parser(report, problem) if parser else None


def problem_name(problem: dict) -> str | None:
    pattern = problem.get("pattern")
    if pattern == "hub":
        evidence = {row.get("kind") for row in problem.get("evidence", [])}
        if "fan_in" in evidence:
            return "everything depends on this"
        if "fan_out" in evidence:
            return "depends on many files"
        return "change spreads far"
    return {
        "god_file": "does too much",
        "tangle": "circular dependency",
        "hot_mess": "hot and complex",
        "shotgun_pair": "packages change together",
        "bus_risk": "one author",
        "unstable_dependency": "depends on less stable code",
        "leaky_interface": "importers follow its changes",
        "hidden_coupling": "change together without a dependency",
    }.get(pattern)


def finding_identity(report: dict, finding: dict) -> str:
    path = finding_path(report, finding)
    name = finding["name"]
    if name.startswith("<") and name.endswith(">"):
        identity = path.rsplit("/", 1)[-1]
    elif finding.get("container"):
        identity = f"{finding['container']}::{name}"
    else:
        identity = name
    kind = {
        "synthetic_top_level": "top level",
    }.get(finding["kind"], finding["kind"])
    facts = []
    if finding["role"] != "primary":
        facts.append(finding["role"])
    if finding["trust"] == "advisory":
        facts.append("advisory")
    suffix = "" if not facts else f" · {' · '.join(facts)}"
    return f"{identity} · {kind}{suffix}"


def problem_head(report: dict, problem: dict) -> str | None:
    anchor = problem_anchor(report, problem)
    rating = problem.get("rating")
    if not anchor or rating not in {"healthy", "watch", "high"}:
        return None
    name = problem_name(problem)
    if problem.get("pattern") == "measured":
        evidence = problem.get("evidence", [])
        if not evidence or evidence[0].get("kind") != "findings":
            return None
        index = evidence[0].get("index")
        if not valid_index(index, report["findings"]):
            return None
        finding = report["findings"][index]
        name = f"{finding_identity(report, finding)} · {anchor}"
    if not name:
        return None
    prefix = "" if rating == "healthy" else f"{rating} "
    return f"{prefix}{name}" if problem.get("pattern") == "measured" else f"{prefix}{name} · {anchor}"


def first_problem_review(report: dict, terminal: str) -> tuple[dict, list[dict]] | None:
    problems = default_problems(report)
    if not problems or not displayed_problem_heads_match(report, terminal):
        return None
    problem = problems[0]
    findings = []
    for claim in problem.get("claimed", []):
        index = claim.get("index")
        if (
            claim.get("table") == "findings"
            and isinstance(index, int)
            and 0 <= index < len(report["findings"])
        ):
            findings.append(report["findings"][index])
    return problem, findings


def displayed_problem_heads_match(report: dict, terminal: str) -> bool:
    problems = default_problems(report)[:24]
    expected = [problem_head(report, problem) for problem in problems]
    lines = section_lines(terminal, "PROBLEMS")
    if None in expected or lines is None:
        return False
    offset = 0
    for head in expected:
        offset = consume_problem_head(lines, offset, head)
        if offset is None:
            return False
    return offset == len(lines)


def consume_problem_head(lines: list[str], offset: int, expected: str) -> int | None:
    if offset >= len(lines) or not lines[offset].startswith(("  high ", "  watch ")):
        return None
    parsed = lines[offset].strip()
    offset += 1
    while parsed != expected:
        if not expected.startswith(parsed) or offset >= len(lines):
            return None
        continuation = lines[offset]
        if not continuation.startswith("        ") or continuation.startswith("         "):
            return None
        text = continuation.strip()
        parsed = f"{parsed}{text}" if parsed.endswith("/") else f"{parsed} {text}"
        offset += 1
    return offset



def codebase_head_matches_report(report: dict, terminal: str) -> bool:
    lines = terminal.splitlines()
    verdict = report.get("verdict", {})
    summary = report.get("summary", {})
    expected_counts = (
        f"{summary.get('high')} high · {summary.get('watch')} watch · "
        f"{summary.get('checked'):,} checked"
    )
    if len(lines) < 3 or lines[0] != "smackdebt · repository root":
        return False
    if verdict.get("sentence") != CODEBASE_TIER_SENTENCES.get(verdict.get("tier")):
        return False
    if lines[1] != f"  {verdict.get('sentence')}":
        return False
    try:
        count_index = lines.index(expected_counts)
    except ValueError:
        return False
    expected_facts = []
    qualifier = verdict.get("qualifier")
    if isinstance(qualifier, dict):
        expected_facts.extend(
            f"  {qualifier.get(key)}" for key in ("sentence", "detail")
        )
    share = verdict.get("share")
    if isinstance(share, dict):
        expected_facts.append(f"  {share.get('sentence')}")
    if lines[2:count_index] != expected_facts:
        return False
    worst = summary.get("worst", [])
    if worst:
        reason = {
            "hot_and_complex": "hot AND complex",
            "most_complex": "most complex",
            "package_dependency_cycle": "package dependency cycle",
        }.get(worst[0].get("reason"))
        expected = f"worst: {worst[0].get('path')} — {reason}"
        if reason is None or count_index + 1 >= len(lines) or lines[count_index + 1] != expected:
            return False
    elif count_index + 1 < len(lines) and lines[count_index + 1].startswith("worst:"):
        return False
    return True


def codebase_sections_match_report(report: dict, terminal: str) -> bool:
    if any(f"\n{heading}\n" in terminal for heading in RETIRED_CODEBASE_HEADINGS):
        return False
    if "\x1b" in terminal or any("\ue000" <= character <= "\uf8ff" for character in terminal):
        return False
    for heading in CODEBASE_HEADINGS:
        if terminal.count(f"\n{heading}\n") > 1:
            return False
        lines = section_lines(terminal, heading)
        if lines == []:
            return False
    if not problem_references_are_valid(report):
        return False
    if not displayed_problem_heads_match(report, terminal):
        return False
    if not warnings_match_report(report, terminal):
        return False
    root = report["scopes"][report["root"]]
    affected_children = [
        report["scopes"][child]
        for child in root["children"]
        if report["health"][report["scopes"][child]["health"]]["high"]
        + report["health"][report["scopes"][child]["health"]]["watch"]
        > 0
    ]
    areas = terminal.find("\nAREAS\n")
    problems = terminal.find("\nPROBLEMS\n")
    warnings = terminal.find("\nWARNINGS\n")
    expected_areas = len(affected_children) >= 2
    return (
        (areas >= 0) == expected_areas
        and (not expected_areas or areas < problems)
        and (warnings < 0 or problems < warnings)
    )


def substantial_trusted_finding(finding: dict | None) -> bool:
    return bool(
        finding
        and finding["trust"] == "trusted"
        and finding["rating"] == "high"
        and finding["measurements"]["cognitive_complexity"] >= 25
    )


def first_problem_has_finding(report: dict, terminal: str, predicate) -> bool:
    review = first_problem_review(report, terminal)
    return bool(review and any(predicate(finding) for finding in review[1]))


def important_debt_leads(report: dict, terminal: str) -> bool:
    return (
        codebase_head_matches_report(report, terminal)
        and codebase_sections_match_report(report, terminal)
        and first_problem_has_finding(report, terminal, substantial_trusted_finding)
    )


def primary_application_finding_leads(report: dict, terminal: str) -> bool:
    return (
        codebase_head_matches_report(report, terminal)
        and codebase_sections_match_report(report, terminal)
        and first_problem_has_finding(
            report,
            terminal,
            lambda finding: substantial_trusted_finding(finding)
            and finding["role"] == "primary",
        )
    )


def useful_rust_finding_leads(report: dict, terminal: str) -> bool:
    return (
        codebase_head_matches_report(report, terminal)
        and codebase_sections_match_report(report, terminal)
        and first_problem_has_finding(
            report,
            terminal,
            lambda finding: substantial_trusted_finding(finding)
            and finding_path(report, finding).endswith(".rs"),
        )
    )


def empty_optional_sections_are_absent(report: dict, terminal: str) -> bool:
    return codebase_sections_match_report(report, terminal)


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


def architecture_claim_is_strong(report: dict, index: int) -> bool:
    if not valid_index(index, report["architecture_findings"]):
        return False
    witness = report["architecture_findings"][index]["witness_edges"]
    if not witness:
        return False
    edges = report["dependency_edges"]
    return all(
        valid_index(edge, edges)
        and edges[edge]["relation"] == "uses"
        and edges[edge]["role"] == "primary"
        and edges[edge]["trust"] == "trusted"
        for edge in witness
    )


def history_claim_is_strong(report: dict, index: int) -> bool:
    if not valid_index(index, report["evolutionary_findings"]):
        return False
    finding = report["evolutionary_findings"][index]
    shared = finding["shared_commits"]
    union = finding["union_commits"]
    return shared >= 3 and union >= shared and shared * 5 >= union


def displayed_graph_and_history_claims_are_strong(report: dict) -> bool:
    checks = {
        "architecture_findings": architecture_claim_is_strong,
        "evolutionary_findings": history_claim_is_strong,
    }
    for problem in default_problems(report)[:24]:
        for claim in problem["claimed"]:
            check = checks.get(claim["table"])
            if check and not check(report, claim["index"]):
                return False
    return True


def weak_history_and_graph_facts_are_absent(report: dict, terminal: str) -> bool:
    return codebase_sections_match_report(
        report, terminal
    ) and displayed_graph_and_history_claims_are_strong(report)


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
    schema = json.loads((ROOT / "schemas" / "report-v4.schema.json").read_text())
    self_report, self_terminal = reviewed_report(binary, args.self_repository, schema)
    mixed_report, mixed_terminal = reviewed_report(binary, args.mixed, schema)
    rust_report, rust_terminal = reviewed_report(binary, args.rust, schema)

    for family, report in zip(FAMILIES, (self_report, mixed_report, rust_report)):
        require(package_references_are_valid(report), f"{family}: package references are invalid")
        require(problem_references_are_valid(report), f"{family}: problem references are invalid")

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
