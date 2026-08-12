#!/bin/sh
set -eu

if [ "$#" -lt 2 ]; then
    echo "usage: $0 cachegrind|dhat OUTPUT_DIR -- command args..." >&2
    exit 2
fi

tool=$1
output=$2
shift 2
[ "${1:-}" = "--" ] || { echo "profile command must follow --" >&2; exit 2; }
shift
[ "$#" -gt 0 ] || { echo "profile command is required" >&2; exit 2; }

command -v valgrind >/dev/null 2>&1 || {
    echo "valgrind is not installed; skipping $tool profile" >&2
    exit 77
}
mkdir -p "$output"
case "$tool" in
    cachegrind)
        exec valgrind --tool=cachegrind --cachegrind-out-file="$output/cachegrind.out" "$@"
        ;;
    dhat)
        exec valgrind --tool=dhat --dhat-out-file="$output/dhat.out" "$@"
        ;;
    *)
        echo "unknown profiler: $tool" >&2
        exit 2
        ;;
esac
