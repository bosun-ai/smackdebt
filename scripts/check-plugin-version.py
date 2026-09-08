"""Require plugin version increases when shipped plugin files change."""

import json
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
PLUGIN = "plugins/smackdebt"
MANIFESTS = (".codex-plugin/plugin.json", ".claude-plugin/plugin.json")
SEMVER = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)


def prerelease_order(prerelease):
    if prerelease is None:
        return ()
    parts = prerelease.split(".")
    if any(part.isdigit() and len(part) > 1 and part.startswith("0") for part in parts):
        raise ValueError(f"invalid plugin prerelease: {prerelease}")
    return tuple((0, int(part)) if part.isdigit() else (1, part) for part in parts)


def version_order(value):
    match = SEMVER.fullmatch(value)
    if not match:
        raise ValueError(f"invalid plugin version: {value}")
    major, minor, patch, prerelease = match.groups()
    return int(major), int(minor), int(patch), prerelease is None, prerelease_order(prerelease)


def git(root, *arguments):
    return subprocess.run(["git", *arguments], cwd=root, text=True, capture_output=True)


def check(root, base):
    revision = git(root, "rev-parse", "--verify", f"{base}^{{commit}}")
    if revision.returncode:
        raise ValueError("cannot read plugin comparison commit")
    base = revision.stdout.strip()
    previous = git(root, "ls-tree", base, "--", PLUGIN)
    if previous.returncode:
        raise ValueError("cannot read previous plugin files")
    if not previous.stdout:
        return
    changes = git(root, "diff", "--quiet", base, "--", PLUGIN)
    if changes.returncode == 0:
        return
    if changes.returncode != 1:
        raise ValueError("cannot compare plugin files")
    for manifest in MANIFESTS:
        path = f"{PLUGIN}/{manifest}"
        before = git(root, "show", f"{base}:{path}")
        if before.returncode:
            raise ValueError(f"cannot read previous {path}")
        old = json.loads(before.stdout)["version"]
        new = json.loads((root / path).read_text())["version"]
        if version_order(new) <= version_order(old):
            raise ValueError(f"increase {path} version above {old} when plugin files change")


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else ""
    if not base or set(base) == {"0"}:
        print("plugin versions: no previous commit to compare")
        return 0
    try:
        check(ROOT, base)
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"plugin versions: {error}", file=sys.stderr)
        return 1
    print("plugin versions: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
