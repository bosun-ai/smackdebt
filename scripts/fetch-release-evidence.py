"""Fetch the exact candidate commit when a GitHub merge rewrote its history."""

import json
import subprocess
from pathlib import Path


def main():
    root = Path(__file__).resolve().parents[1]
    record = json.loads((root / "benchmarks/evidence/workload-reviews.json").read_text())
    revision = record["workspace_revision"]
    if (
        not isinstance(revision, str)
        or len(revision) != 40
        or any(character not in "0123456789abcdef" for character in revision)
    ):
        raise SystemExit("release evidence must name a full Git commit id")
    available = subprocess.run(
        ["git", "cat-file", "-e", f"{revision}^{{commit}}"],
        cwd=root,
        capture_output=True,
    )
    if available.returncode:
        subprocess.run(["git", "fetch", "--no-tags", "origin", revision], cwd=root, check=True)


if __name__ == "__main__":
    main()
