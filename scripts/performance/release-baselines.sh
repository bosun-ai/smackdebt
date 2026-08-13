#!/bin/sh
set -eu

if [ -n "$(git status --porcelain)" ]; then
    echo "release baselines require a clean workspace" >&2
    exit 1
fi

state=$(mktemp "${TMPDIR:-/tmp}/smackdebt-release-state.XXXXXX")
trap 'rm -f "$state"' EXIT
python3 - "$state" <<'PY'
import json
import platform
import subprocess
import sys

state = {
    "workspace_revision": subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip(),
    "workspace_dirty": False,
    "host": platform.platform(),
    "rustc": subprocess.run(
        ["rustc", "--version"], check=True, capture_output=True, text=True
    ).stdout.strip(),
}
with open(sys.argv[1], "w", encoding="utf-8") as stream:
    json.dump(state, stream)
PY

for profile in one-file hundred-file small-diff graph-sparse graph-dense many-package evolution-dense large-dependency-diff; do
    scripts/performance/baseline.sh "$profile" "$state"
done
python3 scripts/performance/check-baselines.py --release-head
