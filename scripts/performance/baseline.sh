#!/bin/sh
set -eu

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
    echo "usage: $0 PROFILE [RELEASE_STATE] [--accept-report-change]" >&2
    exit 2
fi

profile=$1
release_state=${2:-}
accept_report_change=${3:-}
if [ "$release_state" = "--accept-report-change" ]; then
    accept_report_change=$release_state
    release_state=
fi
if [ -n "$accept_report_change" ] && [ "$accept_report_change" != "--accept-report-change" ]; then
    echo "unknown option: $accept_report_change" >&2
    exit 2
fi
case "$profile" in
    one-file|hundred-file|graph-sparse|graph-dense|many-package) command="scripts/performance/smackdebt-codebase.sh" ;;
    evolution-dense|evolution-wide) command="scripts/performance/smackdebt-evolution.sh" ;;
    small-diff|large-dependency-diff) command="scripts/performance/smackdebt-diff.sh" ;;
    *) echo "unknown profile: $profile" >&2; exit 2 ;;
esac

cargo build --release --features allocation-stats
baseline="benchmarks/baselines/$profile.json"
expected_digest=
if [ -f "$baseline" ]; then
    expected_digest=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("report_digest", ""))' "$baseline")
fi
if [ "$accept_report_change" = "--accept-report-change" ]; then
    expected_digest=
fi
if [ -z "$expected_digest" ] && [ "$accept_report_change" != "--accept-report-change" ]; then
    echo "$baseline has no checked report digest; pass --accept-report-change for an intentional update" >&2
    exit 1
fi
output=$(mktemp -d "${TMPDIR:-/tmp}/smackdebt-$profile.XXXXXX")
trap 'rm -rf "$output"' EXIT
digest_arguments=
if [ -n "$expected_digest" ]; then
    digest_arguments="--expected-digest $expected_digest"
fi
# shellcheck disable=SC2086
scripts/performance/run.sh \
    --profile "$profile" \
    --output "$output" \
    --repeat 5 \
    --quiet \
    --check-report \
    $digest_arguments \
    -- "$command"
mkdir -p benchmarks/evidence
cp "$output/metadata.json" "benchmarks/evidence/$profile.metadata.json"
cp "$output/runs.jsonl" "benchmarks/evidence/$profile.runs.jsonl"
state_arguments=
if [ -n "$release_state" ]; then
    state_arguments="--release-state $release_state"
fi
# shellcheck disable=SC2086
python3 scripts/performance/record-baseline.py \
    --profile "$profile" \
    --input "$output" \
    --output "$baseline" \
    $state_arguments
