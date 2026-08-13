#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 one-file|hundred-file|small-diff|graph-sparse|graph-dense|many-package|evolution-dense|large-dependency-diff" >&2
    exit 2
fi

profile=$1
case "$profile" in
    one-file|hundred-file|graph-sparse|graph-dense|many-package) command="scripts/performance/smackdebt-codebase.sh" ;;
    evolution-dense) command="scripts/performance/smackdebt-evolution.sh" ;;
    small-diff|large-dependency-diff) command="scripts/performance/smackdebt-diff.sh" ;;
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
python3 scripts/performance/record-baseline.py \
    --profile "$profile" \
    --input "$output" \
    --output "benchmarks/baselines/$profile.json"
