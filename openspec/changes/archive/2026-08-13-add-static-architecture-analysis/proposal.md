## Why

Source complexity answers where individual code is difficult, but it cannot
show whether package relationships make changes risky. Smackdebt currently uses
packages only for report grouping and does not model imports, dependency cycles,
incoming pressure, outgoing pressure, or architecture changes.

The same language implementations that understand source syntax can extract
dependency references without leaking syntax into graph policy. Adding a flat
dependency graph lets the default and diff commands answer code and architecture
questions in one local report.

## What Changes

- Extract language-specific dependency references through the private language
  contract introduced by `replace-source-analysis-engine`.
- Resolve internal targets through project-owned repository facts without file
  I/O inside language implementations.
- Store file dependency edges once and aggregate package edges from them.
- Add focused graph algorithms for strongly connected components, cycle
  witnesses, fan-in, fan-out, instability, and architecture comparison.
- Integrate separate architecture sections into default and diff terminal
  reports.
- Extend JSON schema version 2 with complete dependency and architecture facts.
- Keep unresolved and ambiguous references visible instead of guessing.

## Capabilities

### New Capabilities

- `architecture-analysis`: Defines static dependency extraction, resolution,
  graph measurements, findings, comparison, and presentation.

### Modified Capabilities

- `workspace-architecture`: Assigns syntax extraction, I/O, resolution, graph
  policy, orchestration, and rendering to their existing crate owners.
- `analysis-performance`: Adds single-pass edge extraction, flat graph storage,
  deterministic graph work, and complete-flow evidence.
- `progressive-exploration`: Integrates architecture sections and progressive
  graph detail into codebase and diff reports.
- `report-schema-v2`: Adds dependency, package graph, finding, and comparison
  tables.
- `architecture-documentation`: Documents the dependency data flow and graph
  algorithms.
- `product-documentation`: Explains architecture output and its limits.

## Impact

This change depends on `replace-source-analysis-engine`. It changes language
semantic output, project resolution, analysis-owned report values, terminal and
JSON output, generated fixtures, performance baselines, README examples, and
architecture documentation. It adds no command or runtime plugin interface.
