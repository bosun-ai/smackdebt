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
"$@" "$output" >/dev/null 2>/dev/null

run=1
while [ "$run" -le "$repeat" ]; do
    stderr_file="$output/run-$run.stderr"
    started=$(python3 -c 'import time; print(time.perf_counter_ns())')
    python3 "$script_dir/timed_command.py" "$stderr_file" "$quiet" "$@" "$output"
    finished=$(python3 -c 'import time; print(time.perf_counter_ns())')
    python3 - "$output" "$run" "$started" "$finished" "$stderr_file" <<'PY'
import json
import pathlib
import re
import sys

root, run, started, finished, stderr_file = sys.argv[1:]
path = pathlib.Path(root) / "runs.jsonl"
stderr = pathlib.Path(stderr_file).read_text()
match = re.search(r'smackdebt parser stats: \{"parser_time_ns":(\d+)\}', stderr)
allocations = re.search(r'allocations: (\d+).*?reallocations: (\d+).*?bytes_allocated: (\d+)', stderr)
resident = re.search(r'smackdebt runner stats: \{"peak_resident_bytes":(\d+)\}', stderr)
project = re.search(r'smackdebt project stats: (\{[^\n]+\})', stderr)
project_stats = json.loads(project.group(1)) if project else {}
record = {
    "run": int(run),
    "wall_time_ns": int(finished) - int(started),
    "parser_time_ns": int(match.group(1)) if match else None,
    "allocation_count": int(allocations.group(1)) if allocations else None,
    "reallocation_count": int(allocations.group(2)) if allocations else None,
    "allocated_bytes": int(allocations.group(3)) if allocations else None,
    "peak_resident_bytes": int(resident.group(1)) if resident else None,
    "inventory_walks": project_stats.get("inventory_walks"),
    "inventory_visits": project_stats.get("inventory_visits"),
    "source_reads": project_stats.get("source_reads"),
    "git_processes": project_stats.get("git_processes"),
}
with path.open("a") as stream:
    stream.write(json.dumps(record, sort_keys=True) + "\n")
PY
    rm -f "$stderr_file"
    run=$((run + 1))
done
