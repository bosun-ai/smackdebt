#!/usr/bin/env python3
"""Require an evidence owner for every accepted analysis spec file."""

from pathlib import Path


ROOT = Path(__file__).parents[1]
CHANGES = (
    "replace-source-analysis-engine",
    "add-static-architecture-analysis",
    "add-evolutionary-architecture-analysis",
    "prove-unified-analysis",
)


def main() -> int:
    evidence = (ROOT / "docs/analysis-evidence.md").read_text()
    missing = []
    for change in CHANGES:
        spec_root = ROOT / "openspec" / "changes" / change / "specs"
        for spec in sorted(spec_root.glob("*/spec.md")):
            identity = f"{change}/specs/{spec.parent.name}/spec.md"
            if f"`{identity}`" not in evidence:
                missing.append(identity)
    if missing:
        for identity in missing:
            print(f"analysis evidence: missing {identity}")
        return 1
    print("analysis evidence: all accepted spec areas mapped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
