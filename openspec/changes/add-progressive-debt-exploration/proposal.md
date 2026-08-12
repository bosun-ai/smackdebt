## Why

Smackdebt already calculates health for repository, package, directory, and file
scopes, but its terminal report jumps from repository totals to individual
findings. Users cannot see which areas own the debt or move through the project
one useful level at a time. Diff reports have the same gap and do not retain the
links needed to aggregate changed units by directory.

## What Changes

- Make codebase and diff reports progressively explorable from repository to
  package, directory, file, container, and code unit.
- Show exact child health or change counts and each child's share of the selected
  scope, without introducing a combined score.
- Keep scope selection separate from the report so one analyzed report can power
  several terminal views and a later interactive terminal interface.
- Preserve repository-relative identities when a path limits analysis, while
  keeping source reads limited to that selection.
- Extend JSON schema version 1 with additive navigation and diff ownership fields.
- Add concise default output, smart single-child skipping, and an `--all` terminal
  option for complete rows and retained details.

## Capabilities

### New Capabilities

- `progressive-exploration`: Defines scope navigation, distribution tables,
  selected-scope detail, diff aggregation, terminal limits, and JSON additions.

### Modified Capabilities

- `workspace-architecture`: Defines shared path ownership, scope selection, and
  comparison ownership needed by any renderer.
- `analysis-performance`: Keeps path-limited work proportional to the selection
  and proves that changing views does not rerun analysis.
- `product-documentation`: Requires realistic codebase and diff exploration
  examples before release behavior is frozen.
- `architecture-documentation`: Records navigation ownership and aggregation
  rules for future renderers.

## Impact

The implementation will change report values in `smackdebt-analysis`, hierarchy
construction in `smackdebt-project`, terminal and JSON rendering in
`smackdebt-output`, and CLI arguments in `smackdebt`. It adds fields to JSON
schema version 1 but does not remove or reinterpret existing fields. It does not
add a terminal interface, cache, server, or new source measurement.
