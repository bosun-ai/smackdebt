from __future__ import annotations


def valid_index(value, rows: list) -> bool:
    return isinstance(value, int) and 0 <= value < len(rows)


def warning_lines(terminal: str) -> list[str] | None:
    marker = "\nWARNINGS\n"
    if marker not in terminal:
        return None
    lines = terminal.split(marker, 1)[1].splitlines()
    return [
        line
        for line in lines
        if line and not line.lstrip().startswith("next: ")
    ]


def counted(value: int, singular: str, plural: str) -> str:
    return f"{value:,} {singular if value == 1 else plural}"


def rendered_warning(head: str, facts: list[str] | None = None) -> list[str]:
    facts = facts or []
    joined = " · ".join([head, *facts])
    if 10 + len(joined) <= 100:
        return [f"warning {joined}"]
    lines = [f"warning {head}"]
    joined_facts = " · ".join(facts)
    return lines + ([joined_facts] if 8 + len(joined_facts) <= 100 else facts)


def history_warning_rows(report: dict) -> list[str]:
    history = report["history_coverage"]
    rows = []
    if history["availability"] == "incomplete":
        rows.extend(rendered_warning("History is incomplete."))
    elif history["availability"] == "unavailable":
        rows.extend(rendered_warning("History is unavailable."))
    elif history["eligible_commits"] == 0 and history.get("window_days") is not None:
        days = counted(history["window_days"], "day", "days")
        rows.extend(rendered_warning(f"No commits in the last {days}."))
    if history["rename_gaps"] > 0:
        rows.extend(rendered_warning("Some renamed files could not be matched."))
    return rows


def graph_warning_rows(report: dict) -> list[str]:
    evidence = report["graph_evidence"]
    hidden = sum(
        evidence[key]
        for key in ("suppressed_reach", "suppressed_core", "suppressed_leakage")
    )
    if hidden == 0:
        return []
    facts = counted(hidden, "architecture fact", "architecture facts")
    return rendered_warning(f"{facts} hidden because dependency data is incomplete.")


def resolution_warning_rows(report: dict) -> list[str]:
    diagnostics = report["resolution_diagnostics"]
    unresolved = sum(row["kind"] == "unresolved" for row in diagnostics)
    ambiguous = sum(row["kind"] == "ambiguous" for row in diagnostics)
    total = unresolved + ambiguous
    if total == 0:
        return []
    head = f"{counted(total, 'import', 'imports')} could not be followed"
    facts = []
    if unresolved:
        facts.append(f"{unresolved} named nothing in the repository")
    if ambiguous:
        facts.append(f"{ambiguous} matched more than one file")
    return rendered_warning(head, facts)


DIAGNOSTIC_WORDS = {
    "nested_repository": (
        "nested repository",
        "nested repositories",
        "was not analyzed.",
        "were not analyzed.",
    ),
    "unsupported_language": (
        "source file",
        "source files",
        "uses an unsupported language.",
        "use unsupported languages.",
    ),
    "unreadable_file": ("source file", "source files", "could not be read.", "could not be read."),
    "oversized_file": (
        "source file",
        "source files",
        "is too large to inspect.",
        "are too large to inspect.",
    ),
    "parse_failure": (
        "source file",
        "source files",
        "could not be fully parsed.",
        "could not be fully parsed.",
    ),
    "ambiguous_identity": (
        "file",
        "files",
        "has anonymous units that could not be matched safely.",
        "have anonymous units that could not be matched safely.",
    ),
    "unsafe_reference": (
        "source file",
        "source files",
        "contains an unsafe reference.",
        "contain unsafe references.",
    ),
    "other": ("source file", "source files", "could not be analyzed.", "could not be analyzed."),
}


def diagnostic_affects_root(report: dict, diagnostic: dict) -> bool:
    file = diagnostic.get("file")
    if file is None or not valid_index(file, report["files"]):
        return file is None
    return report["files"][file]["role"] in {"primary", "test", "example", "benchmark"}


def diagnostic_warning_rows(report: dict) -> list[str]:
    rows = []
    for kind, words in DIAGNOSTIC_WORDS.items():
        diagnostics = diagnostics_for_warning(report, kind)
        files = {row["file"] for row in diagnostics if row.get("file") is not None}
        count = len(files) + sum(row.get("file") is None for row in diagnostics)
        if count:
            subject = counted(count, words[0], words[1])
            rows.extend(rendered_warning(f"{subject} {words[2] if count == 1 else words[3]}"))
    return rows


def diagnostics_for_warning(report: dict, kind: str) -> list[dict]:
    return [
        diagnostic
        for diagnostic in report["diagnostics"]
        if diagnostic["kind"] == kind
        and not diagnostic["message"].startswith("Git history")
        and (
            kind != "ambiguous_identity"
            or diagnostic_affects_root(report, diagnostic)
        )
    ]


def warnings_match_report(report: dict, terminal: str) -> bool:
    expected = (
        history_warning_rows(report)
        + graph_warning_rows(report)
        + resolution_warning_rows(report)
        + diagnostic_warning_rows(report)
    )
    lines = warning_lines(terminal)
    if lines is None:
        return not expected
    return [line.strip() for line in lines] == expected

