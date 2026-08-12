#!/bin/sh
set -eu

if [ "$#" -lt 4 ]; then
    echo "usage: $0 --repo PRIVATE_REPOSITORY --output METADATA.json -- command args..." >&2
    exit 2
fi

repo=
output=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --repo) repo=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        --) shift; break ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done
[ -n "$repo" ] || { echo "--repo is required" >&2; exit 2; }
[ -n "$output" ] || { echo "--output is required" >&2; exit 2; }
[ "$#" -gt 0 ] || { echo "benchmark command is required" >&2; exit 2; }
[ -d "$repo" ] || { echo "repository does not exist: $repo" >&2; exit 2; }

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$(dirname -- "$output")"
python3 "$script_dir/workload.py" metadata --private --input "$repo" --output "$output"
started=$(python3 -c 'import time; print(time.perf_counter_ns())')
(CDPATH= cd -- "$repo" && "$@")
finished=$(python3 -c 'import time; print(time.perf_counter_ns())')
python3 - "$output" "$started" "$finished" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
document = json.loads(path.read_text())
document["wall_time_ns"] = int(sys.argv[3]) - int(sys.argv[2])
path.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")
PY
