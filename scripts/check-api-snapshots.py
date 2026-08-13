#!/usr/bin/env python3
"""Compare source-level public declaration snapshots for workspace libraries."""

from __future__ import annotations

import argparse
import difflib
import subprocess
import sys
from pathlib import Path


REQUIRED_TOOL_VERSION = "cargo-public-api 0.52.0"
LIBRARIES = {
    "smackdebt-analysis": Path("crates/analysis/src/lib.rs"),
    "smackdebt-languages": Path("crates/languages/src/lib.rs"),
    "smackdebt-discovery": Path("crates/discovery/src/lib.rs"),
    "smackdebt-git": Path("crates/git/src/lib.rs"),
    "smackdebt-project": Path("crates/project/src/lib.rs"),
    "smackdebt-output": Path("crates/output/src/lib.rs"),
}


def public_surface(source: str) -> list[str]:
    declarations: list[str] = []
    pending: list[str] | None = None
    for line in source.splitlines() + [""]:
        stripped = line.strip()
        if pending is None:
            if not stripped.startswith("pub ") or stripped.startswith(("pub(crate)", "pub(super)", "pub(in ")):
                continue
            pending = [stripped]
        else:
            pending.append(stripped)
        joined = " ".join(part for part in pending if part)
        delimiter = declaration_delimiter(joined)
        if delimiter >= 0:
            declarations.append(" ".join(joined[:delimiter].split()))
            pending = None
    return sorted(set(declarations))


def declaration_delimiter(declaration: str) -> int:
    """Find a declaration terminator outside parameter and type brackets."""
    opening = {"[": "]", "(": ")", "<": ">"}
    closing = set(opening.values())
    depth = 0
    for position, character in enumerate(declaration):
        if character in opening:
            depth += 1
        elif character in closing:
            depth = max(0, depth - 1)
        elif depth == 0 and character in "{;,":
            return position
    return -1


def snapshot_path(root: Path, package: str, all_features: bool) -> Path:
    suffix = ".all-features" if all_features else ""
    return root / "api-snapshots" / f"{package}{suffix}.txt"


def tool_problem(root: Path) -> str | None:
    try:
        completed = subprocess.run(
            ["cargo", "public-api", "--version"],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return "install cargo-public-api 0.52.0 with `cargo install cargo-public-api --version 0.52.0 --locked`"
    if completed.stdout.strip() != REQUIRED_TOOL_VERSION:
        return f"expected {REQUIRED_TOOL_VERSION}, found {completed.stdout.strip() or 'unknown version'}"
    return None


def reachable_api(root: Path, package: str, all_features: bool = False) -> list[str]:
    feature_arguments = ["--all-features"] if all_features else []
    completed = subprocess.run(
        [
            "cargo",
            "public-api",
            "-p",
            package,
            *feature_arguments,
            "-sss",
            "--color",
            "never",
        ],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.splitlines()


def check(
    root: Path,
    update: bool,
    selected: set[str] | None = None,
    all_features: bool = False,
) -> list[str]:
    problem = tool_problem(root)
    if problem is not None:
        return [problem]
    problems: list[str] = []
    for package, relative_source in LIBRARIES.items():
        if selected is not None and package not in selected:
            continue
        source_path = root / relative_source
        expected_path = snapshot_path(root, package, all_features)
        if not source_path.exists():
            problems.append(f"{package}: missing library source {relative_source}")
            continue
        try:
            actual = (
                reachable_api(root, package, True)
                if all_features
                else reachable_api(root, package)
            )
        except subprocess.CalledProcessError as error:
            detail = error.stderr.strip() or error.stdout.strip() or str(error)
            problems.append(f"{package}: API inspection failed: {detail}")
            continue
        if update:
            expected_path.parent.mkdir(parents=True, exist_ok=True)
            expected_path.write_text(("\n".join(actual) + "\n") if actual else "", encoding="utf-8")
            continue
        if not expected_path.exists():
            problems.append(f"{package}: missing snapshot {expected_path.relative_to(root)}")
            continue
        expected = expected_path.read_text(encoding="utf-8").splitlines()
        if expected != actual:
            diff = "\n".join(
                difflib.unified_diff(
                    expected,
                    actual,
                    fromfile=str(expected_path.relative_to(root)),
                    tofile=str(relative_source),
                    lineterm="",
                )
            )
            problems.append(f"{package}: public API changed\n{diff}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--update", action="store_true")
    parser.add_argument("--package", action="append", choices=sorted(LIBRARIES))
    parser.add_argument("--all-features", action="store_true")
    parser.add_argument("--root", type=Path, default=Path("."))
    arguments = parser.parse_args()
    root = arguments.root.resolve()
    problems = check(
        root,
        arguments.update,
        set(arguments.package) if arguments.package else None,
        arguments.all_features,
    )
    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1
    print("API snapshots: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
