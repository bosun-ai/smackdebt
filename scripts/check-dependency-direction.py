#!/usr/bin/env python3
"""Check the non-development workspace dependency graph."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


ALLOWED_EDGES = {
    "smackdebt-analysis": set(),
    "smackdebt-languages": {"smackdebt-analysis"},
    "smackdebt-discovery": {"smackdebt-analysis"},
    "smackdebt-git": {"smackdebt-analysis"},
    "smackdebt-project": {
        "smackdebt-analysis",
        "smackdebt-discovery",
        "smackdebt-git",
        "smackdebt-languages",
    },
    "smackdebt-output": {"smackdebt-analysis"},
    "smackdebt": {"smackdebt-output", "smackdebt-project"},
}


def violations(metadata: dict[str, Any]) -> list[str]:
    workspace_names = {package["name"] for package in metadata["packages"]}
    problems: list[str] = []
    for package in metadata["packages"]:
        package_name = package["name"]
        allowed = ALLOWED_EDGES.get(package_name)
        if allowed is None:
            problems.append(f"unknown workspace crate {package_name}")
            continue
        for dependency in package["dependencies"]:
            if dependency.get("kind") not in (None, "normal"):
                continue
            dependency_name = dependency.get("rename") or dependency["name"]
            if dependency_name in workspace_names and dependency_name not in allowed:
                problems.append(
                    f"{package_name} -> {dependency_name} is not an allowed production edge"
                )
    return problems


def read_metadata(root: Path) -> dict[str, Any]:
    completed = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest-path", type=Path)
    arguments = parser.parse_args()
    root = (arguments.manifest_path or Path("Cargo.toml")).resolve().parent
    problems = violations(read_metadata(root))
    if problems:
        for problem in problems:
            print(f"dependency direction: {problem}", file=sys.stderr)
        return 1
    print("dependency direction: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
