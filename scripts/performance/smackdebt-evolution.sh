#!/bin/sh
set -eu
export SMACKDEBT_ALLOCATION_STATS=1
binary=$(pwd)/target/release/smackdebt
cd "$1"
case "${SMACKDEBT_PERF_JOBS:-auto}" in
    auto) exec "$binary" --json --history 36500d . ;;
    *) exec "$binary" --json --history 36500d --jobs "$SMACKDEBT_PERF_JOBS" . ;;
esac
