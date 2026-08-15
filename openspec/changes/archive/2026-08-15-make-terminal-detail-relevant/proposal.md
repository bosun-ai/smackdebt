> Superseded on 2026-08-15 by the v-next change set (`resolve-workspace-dependencies`, `deepen-debt-signals`, `add-verdict-policy`, `redesign-terminal-report`, `adopt-report-schema-v4`); archived with `--skip-specs` at 0% implementation, so none of its deltas entered accepted specs.

## Why

The clear terminal verdict still expands into facts that do not help a person
decide what debt to inspect. `--all` and path selection currently promise raw
architecture relationships, weak history observations, and activity context,
so a request for useful detail becomes a report-data dump. The second terminal
change must make default and expanded human views consistently debt-focused
while JSON remains the complete interface.

## What Changes

- Apply one relevance policy to repository, package, directory, and file views:
  at most five debt-bearing areas, three ranked source findings, three
  architecture findings with closed witnesses, three actionable history
  findings, grouped warnings, and one discover command.
- Make path selection change scope only; it does not enable raw relationships,
  weak history, activity, concentration, or healthy detail.
- Define `--all` as all useful debt: remove limits for debt-bearing areas,
  retained Watch/High source findings, architecture findings with witnesses,
  actionable history findings, and real failed/recovered diagnostics.
- Keep raw edges, ownership, external/unmatched/ambiguous rows, weak or explained
  history, activity, concentration, and healthy rows out of every human view.
- Keep source finding commit counts, closed witness evidence, and actionable
  coupling evidence, ending human coupling rows with `not linked in code`.
- Make analysis/report aggregation own unique typed IDs in every scope link
  list; reject duplicate IDs in index audits and make the renderer trust and
  visit each completed link once without a de-dup set or identity mapping.
- Review exact default and `--all` scope snapshots at every accepted width;
  apply nonempty-line budgets only to default public codebase/path results and
  never cap useful-debt `--all`.
- Preserve the complete report and JSON version 3, analysis, CLI flags, rank,
  work, allocation, and resource behavior.

## Capabilities

### Modified Capabilities

- `terminal-output`: Defines one default policy, useful `--all`, path behavior,
  unique-link trust, section presence, and exact actionable-history wording.
- `progressive-exploration`: Makes every scope use the same limits and existing
  links without turning path selection into a detail switch.
- `architecture-analysis`: Limits human architecture to findings, closed
  witnesses, and the grouped resolution warning while JSON remains complete.
- `evolutionary-analysis`: Limits human history to actionable finding IDs and
  keeps weak, explained, activity, coverage-field, and concentration facts in
  JSON.
- `analysis-performance`: Requires one-pass borrowed selection from existing
  links with no project work, repeated scans, or large clones.
- `workspace-architecture`: Makes report aggregation own unique typed IDs in
  every scope link list and makes index integrity reject repeated IDs.
- `report-schema-v3`: Extends executable index checks to repeated IDs within
  one owning link list while preserving accepted exact JSON bytes.
- `product-documentation`: Defines `--all` as all useful debt and JSON as the
  complete retained report.
- `architecture-documentation`: Documents the terminal selection seam and its
  separation from report and JSON facts.
- `end-to-end-evidence`: Adds dense exact scope, width, absence, owning-link
  integrity, default-only line-count, unchanged-machine, and workload proof.
- `release-readiness`: Enforces implementation after the verdict change and
  before the debt-focused diff change.

## Dependency Order

All three terminal changes can be authored now. Implementation, review, and
archive order is `make-terminal-verdict-clear`, then
`make-terminal-detail-relevant`, then `make-diff-output-debt-focused`.
`prepare-first-release` remains blocked until all three are archived with their
reviewed evidence.

## Impact

This changes human terminal, help, README, architecture guidance, and exact
snapshot bytes. It affects output selection and writing plus public and
aggregate workload proof. It adds no CLI flag and changes no report field, JSON
version-3 byte contract, analysis rule, finding rank, exit behavior, worker
policy, or resource limit. Diff direction and debt-focused diff storytelling
remain owned by `make-diff-output-debt-focused`.
