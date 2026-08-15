#!/bin/sh
set -eu

usage() {
    echo "usage: $0 --profile PROFILE --output DIR [--repeat N] [--quiet] [--check-report] [--expected-digest SHA256] [-- command args...]" >&2
}

profile=
output=
repeat=1
quiet=false
check_report=false
expected_digest=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --profile) profile=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        --repeat) repeat=$2; shift 2 ;;
        --quiet) quiet=true; shift ;;
        --check-report) check_report=true; shift ;;
        --expected-digest) expected_digest=$2; shift 2 ;;
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

if [ "$check_report" = true ]; then
    # Prove the measured public flow before entering the measured interval.
    : > "$output/correctness.serial.json"
    : > "$output/correctness.parallel.json"
    : > "$output/correctness.sha256"
    : > "$output/runs.jsonl"
    serial_stderr=$(mktemp "${TMPDIR:-/tmp}/smackdebt-correctness-serial.XXXXXX")
    parallel_stderr=$(mktemp "${TMPDIR:-/tmp}/smackdebt-correctness-parallel.XXXXXX")
    work_evidence=$(mktemp "${TMPDIR:-/tmp}/smackdebt-correctness-work.XXXXXX")
    trap 'rm -f "$serial_stderr" "$parallel_stderr" "$work_evidence"' EXIT
    SMACKDEBT_PERF_JOBS=1 "$@" "$output" > "$output/correctness.serial.json" 2> "$serial_stderr"
    SMACKDEBT_PERF_JOBS=auto "$@" "$output" > "$output/correctness.parallel.json" 2> "$parallel_stderr"
    cmp "$output/correctness.serial.json" "$output/correctness.parallel.json"
    python3 - "$serial_stderr" "$parallel_stderr" "$work_evidence" <<'PY'
import json
import pathlib
import re
import sys

def project_stats(path):
    value = pathlib.Path(path).read_text()
    match = re.search(r'smackdebt project stats: (\{[^\n]+\})', value)
    if match is None:
        raise SystemExit(f"performance correctness: missing live work totals in {path}")
    return json.loads(match.group(1))

serial = project_stats(sys.argv[1])
parallel = project_stats(sys.argv[2])
if serial != parallel:
    raise SystemExit("performance correctness: serial and automatic work totals differ")
pathlib.Path(sys.argv[3]).write_text(json.dumps(serial, sort_keys=True) + "\n")
PY
    schema="$script_dir/../../schemas/report-v4.schema.json"
    digest_arguments=
    if [ -n "$expected_digest" ]; then
        digest_arguments="--expected-digest $expected_digest"
    fi
    # shellcheck disable=SC2086
    python3 "$script_dir/check-report.py" \
        --profile "$profile" \
        --workload "$output/manifest.json" \
        --report "$output/correctness.serial.json" \
        --schema "$schema" \
        --work-evidence "$work_evidence" \
        --digest-output "$output/correctness.sha256" \
        $digest_arguments
fi

# Keep the inventory shape identical for every measured run.
: > "$output/runs.jsonl"

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
    "object_reads": project_stats.get("object_reads"),
    "git_processes": project_stats.get("git_processes"),
    "parser_visits": project_stats.get("parser_visits"),
    "algorithm_passes": project_stats.get("algorithm_passes"),
}
with path.open("a") as stream:
    stream.write(json.dumps(record, sort_keys=True) + "\n")
PY
    rm -f "$stderr_file"
    run=$((run + 1))
done
