#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 one-file|hundred-file|small-diff" >&2
    exit 2
fi

profile=$1
case "$profile" in
    one-file|hundred-file) command="scripts/performance/smackdebt-codebase.sh" ;;
    small-diff) command="scripts/performance/smackdebt-diff.sh" ;;
    *) echo "unknown profile: $profile" >&2; exit 2 ;;
esac

cargo build --release --features allocation-stats
output=$(mktemp -d "${TMPDIR:-/tmp}/smackdebt-$profile.XXXXXX")
trap 'rm -rf "$output"' EXIT
scripts/performance/run.sh \
    --profile "$profile" \
    --output "$output" \
    --repeat 5 \
    --quiet \
    -- "$command"
mkdir -p benchmarks/evidence
cp "$output/metadata.json" "benchmarks/evidence/$profile.metadata.json"
cp "$output/runs.jsonl" "benchmarks/evidence/$profile.runs.jsonl"
