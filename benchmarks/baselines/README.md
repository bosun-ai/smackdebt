# Baseline records

Baseline records belong here after the complete CLI and diff flows pass their
correctness checks. A record must include:

- workload profile, seed, source digest, report digest, and command;
- Git revision and dirty state;
- host and toolchain;
- source bytes and supported files;
- wall time, p95, peak memory, allocations, inventory visits, source and object
  reads, parser visits, algorithm passes, and Git process count;
- the resulting limits and the reason for ten percent regression room.

The source measurements are in `one-file.json`, `hundred-file.json`, and
`small-diff.json`. Static graph measurements add `graph-sparse.json`,
`graph-dense.json`, `many-package.json`, and `large-dependency-diff.json`.
They were recorded from an optimized build after output correctness checks.
Evolutionary analysis adds `evolution-dense.json`, with two commits touching
100 packages and every observed unordered package pair retained once. Both of
its commits exceed the bulk-commit guard, so it is also the proof that a
sweeping commit contributes no file pair.

The `evolution-wide` profile — 2,000 files in 50 packages, 40 commits, a
dependency ring with an isolated tail, one leaky interface, four pairs no
connection path joins, and one sweeping commit — is in the release profile loop
and has no record here yet. Every committed record shares one workspace
revision, so its record is written by the next `just release-baselines` rather
than on its own. Its correctness half runs on every measured invocation.
The 100-file comparison measured five serial runs at
40–50 ms and five four-worker runs at 30–40 ms, so the first parallel cutover
is 100 files.

The allocation counts use the CLI's `allocation-stats` release feature. DHAT
and Cachegrind remain available for deeper profiles on supported hosts.
Wall-time and resident-memory budgets are ten percent above the recorded p95
and peak values.

`just release-baselines` requires a clean tree, captures one starting state,
records every profile, and verifies that all records use that HEAD with
`workspace_dirty` set to false. A report-byte change requires the explicit
`--accept-report-change` baseline option and review of the new digest.
