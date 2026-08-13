# Release evidence

Crates remain private. Publication requires a separate accepted change after
all source, static architecture, evolution, and unified analysis evidence has
passed from the release revision.

Run:

```console
just release-evidence
```

The command requires a clean tree, captures one reviewed HEAD/toolchain/host
state, and records all eight workloads against it. It records workload
identity, report digest, source bytes, supported files, wall-time samples and p95, peak
memory, allocation counts, inventory visits, source and object reads, parser
visits, algorithm passes, and Git processes. The install smoke must prove help,
version, terminal, and JSON behavior from outside the source workspace.

Do not publish when a generated fixture, public flow, schema check, exact
result, install smoke, resource record, or lower-level truth test is missing.
