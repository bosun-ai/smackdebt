#!/bin/sh
set -eu

usage() {
    echo "usage: $0 --profile PROFILE --output DIR [--repeat N] [--quiet] [-- command args...]" >&2
}

profile=
output=
repeat=1
quiet=false
while [ "$#" -gt 0 ]; do
    case "$1" in
        --profile) profile=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        --repeat) repeat=$2; shift 2 ;;
        --quiet) quiet=true; shift ;;
        --) shift; break ;;
        *) usage; exit 2 ;;
    esac
done

if [ -z "$profile" ] || [ -z "$output" ]; then usage; exit 2; fi
case "$repeat" in ''|*[!0-9]*|0) echo "repeat must be a positive integer" >&2; exit 2 ;; esac

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
python3 "$script_dir/workload.py" generate --output "$output" --profile "$profile"
python3 "$script_dir/workload.py" check --input "$output" > "$output/check.json"
python3 "$script_dir/workload.py" metadata --input "$output" --output "$output/metadata.json"

if [ "$#" -eq 0 ]; then
    cat "$output/metadata.json"
    exit 0
fi

# Prime executable loading and parser setup outside the measured interval.
"$@" "$output" >/dev/null

run=1
while [ "$run" -le "$repeat" ]; do
    started=$(python3 -c 'import time; print(time.perf_counter_ns())')
    if [ "$quiet" = true ]; then
        "$@" "$output" >/dev/null
    else
        "$@" "$output"
    fi
    finished=$(python3 -c 'import time; print(time.perf_counter_ns())')
    python3 - "$output" "$run" "$started" "$finished" <<'PY'
import json
import pathlib
import sys

root, run, started, finished = sys.argv[1:]
path = pathlib.Path(root) / "runs.jsonl"
record = {"run": int(run), "wall_time_ns": int(finished) - int(started)}
with path.open("a") as stream:
    stream.write(json.dumps(record, sort_keys=True) + "\n")
PY
    run=$((run + 1))
done
