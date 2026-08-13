# Baseline records

Baseline records belong here after the complete CLI and diff flows pass their
correctness checks. A record must include:

- workload profile, seed, source digest, and command;
- Git revision and dirty state;
- host and toolchain;
- source bytes and supported files;
- wall time, p95, peak memory, allocations, filesystem reads, and Git process count;
- the resulting limits and the reason for ten percent regression room.

The source measurements are in `one-file.json`, `hundred-file.json`, and
`small-diff.json`. Static graph measurements add `graph-sparse.json`,
`graph-dense.json`, `many-package.json`, and `large-dependency-diff.json`.
They were recorded from an optimized build after output correctness checks.
The 100-file comparison measured five serial runs at
40–50 ms and five four-worker runs at 30–40 ms, so the first parallel cutover
is 100 files.

The allocation counts use the CLI's `allocation-stats` release feature. DHAT
and Cachegrind remain available for deeper profiles on supported hosts.
Wall-time and resident-memory budgets are ten percent above the recorded p95
and peak values.
