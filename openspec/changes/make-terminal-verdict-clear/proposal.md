## Why

The short terminal report still makes users calculate the overall result. Its
quality summary repeats a count, area bars imply precision that does not help a
drill decision, fixture/generated exclusions look like analysis failures, and
some narrow rows preserve width by losing useful facts. Common missing-input errors
also expose implementation text instead of naming the input the user can fix.

## What Changes

- Make non-empty `QUALITY` state a grouped checked count, an integer-safe
  nearest-tenth share that needs attention, and exact High and Watch counts in
  two useful lines; use only `Nothing was checked.` when checked count is zero.
- Remove terminal-only area rates, shares, and bars while keeping exact status
  counts and stable severity-led order.
- Group real source-analysis gaps in default root, package, and directory views;
  require `--all` and a selected affected file to name the relevant file once;
  keep fixture/generated exclusions out of gap warnings.
- Combine unmatched and ambiguous imports into the separate exact Warning-glyph
  `<count> import(s) could not be followed.` row.
- Replace fixed width tiers with content-aware rows that stack or shorten
  identities without silently losing measurements, evidence, states, or
  commands.
- Give missing paths, missing Git refs, and the `--all --json` conflict their
  exact one-line `smackdebt:` stderr bytes, required statuses, empty stdout, and
  no usage/help tail, without leaking absolute paths, operating-system errors,
  Git commands, statuses, or fatal output.
- Preserve report facts, JSON version 3, analysis, status classes, stream
  placement, rank, work, and resource behavior.

## Capabilities

### Modified Capabilities

- `terminal-output`: Defines the two-line verdict, useful area counts, grouped
  incomplete-analysis warnings, content-aware writing, and exact errors.
- `progressive-exploration`: Replaces area rate/share display and fixed width
  tiers while preserving report facts and progressive selection.
- `product-documentation`: Makes examples and guidance match the revised human
  verdict, coverage meaning, width behavior, and errors.
- `architecture-documentation`: Documents content-aware terminal presentation
  without moving analysis or policy into output code.
- `architecture-analysis`: Keeps exact resolution coverage in report and JSON
  while grouping uncertain resolution in one human warning.
- `end-to-end-evidence`: Adds exact output, no-loss width, error, unchanged JSON,
  parallelism, and privacy-safe workload proof.
- `release-readiness`: Keeps publication blocked through this change and the two
  following terminal changes.

## Dependency Order

All three changes can be authored now. Their implementation order is:

1. Implement, review, and archive `make-terminal-verdict-clear`.
2. Then implement, review, and archive `make-terminal-detail-relevant`.
3. Then implement, review, and archive `make-diff-output-debt-focused`.
4. Resume `prepare-first-release` only after all three changes are archived and
   their reviewed evidence passes.

## Impact

This changes human terminal, error, documentation, and exact snapshot bytes. It
affects the output crate, CLI input failures, README and architecture guidance,
acceptance fixtures, and release review. It does not change JSON version 3,
report data, analysis policy, rank, discovery, Git analysis, worker behavior,
exit classes, or resource limits. Detailed relationship filtering and the
debt-focused diff redesign remain owned by the two later changes.
