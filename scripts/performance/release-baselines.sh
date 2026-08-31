#!/bin/sh
set -eu

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
    echo "usage: $0 SELF_REPOSITORY MIXED_REPOSITORY RUST_REPOSITORY [--accept-report-change]" >&2
    exit 2
fi
accept_report_change=${4:-}
if [ -n "$accept_report_change" ] && [ "$accept_report_change" != "--accept-report-change" ]; then
    echo "unknown option: $accept_report_change" >&2
    exit 2
fi

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

for profile in one-file hundred-file small-diff graph-sparse graph-dense many-package evolution-dense evolution-wide large-dependency-diff; do
    scripts/performance/baseline.sh "$profile" "$state" $accept_report_change
done
revision=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["workspace_revision"])' "$state")
python3 scripts/performance/review-workloads.py \
    --binary target/release/smackdebt \
    --revision "$revision" \
    --self "$1" \
    --mixed "$2" \
    --rust "$3" \
    --output benchmarks/evidence/workload-reviews.json
