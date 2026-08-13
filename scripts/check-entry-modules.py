#!/usr/bin/env python3
"""Keep Rust entry modules limited to imports and reexports."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).parents[1]
ALLOWED_STARTS = (
    "#![",
    "//!",
    "///",
    "mod ",
    "use ",
    "pub use ",
)
FORBIDDEN = re.compile(
    r"^(?:pub(?:\([^)]*\))?\s+)?(?:fn|struct|enum|trait|type|const|static|macro_rules!|impl)\b"
)


def entry_modules(root: Path) -> list[Path]:
    crates = root / "crates"
    return sorted([*crates.glob("*/src/lib.rs"), *crates.glob("*/src/**/mod.rs")])


def violations(path: Path) -> list[str]:
    problems: list[str] = []
    statement = ""
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith(("//", "#[")):
            continue
        if statement:
            statement += " " + line
            if ";" in line:
                if "::*" in statement:
                    problems.append(f"{path}:{number}: wildcard reexport is not allowed")
                statement = ""
            continue
        if FORBIDDEN.match(line) or line.startswith("mod ") and "{" in line:
            problems.append(f"{path}:{number}: behavior is not allowed in an entry module")
            continue
        if line.startswith(ALLOWED_STARTS):
            if line.startswith(("use ", "pub use ")) and ";" not in line:
                statement = line
            elif line.startswith("pub use ") and "::*" in line:
                problems.append(f"{path}:{number}: wildcard reexport is not allowed")
            elif line.startswith("mod ") and not line.endswith(";"):
                problems.append(f"{path}:{number}: module declaration must be private and external")
            continue
        if line in ("};", "}"):
            continue
        problems.append(f"{path}:{number}: unsupported entry-module statement")
    if statement:
        problems.append(f"{path}: unterminated import or reexport")
    return problems


def check(root: Path) -> list[str]:
    return [problem for path in entry_modules(root) for problem in violations(path)]


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"entry modules: {problem}", file=sys.stderr)
        return 1
    print("entry modules: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
