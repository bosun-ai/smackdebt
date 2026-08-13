# Change: Add evolutionary architecture analysis

## Why

Static source analysis shows what the code looks like now, but it cannot show
where change repeatedly concentrates or which packages tend to move together.
Those signals are needed to find architecture that is expensive in practice,
especially when static dependencies do not explain the relationship.

## What Changes

- Stream Git history once and derive file and package churn.
- Preserve current-file identity across detected renames.
- Measure package change coupling from shared commits.
- Report package contributor count and contributor concentration without
  exposing author identities.
- Add Watch findings for recurring cross-package change coupling that has no
  matching static dependency.
- Add evolutionary sections to codebase and diff reports and JSON version 2.
- Keep history facts separate from source metrics and static graph facts.
- Add generated history fixtures that prove exact values, privacy, failure
  behavior, and process limits.

## Capabilities

### New Capabilities

- `evolutionary-analysis`: Git-history measurements, change coupling, privacy,
  and evolutionary findings.

### Modified Capabilities

- `analysis-performance`: Require one streamed history process and controlled
  relationship storage.
- `progressive-exploration`: Add evolutionary summaries and drill targets.
- `report-schema-v2`: Add history coverage, churn, coupling, concentration,
  and evolutionary comparison tables.
- `architecture-documentation`: Document history ownership, policy, and privacy.
- `product-documentation`: Explain the evolutionary signals and limitations.

## Impact

- Depends on `replace-source-analysis-engine` and
  `add-static-architecture-analysis`.
- Changes `smackdebt-git`, `smackdebt-analysis`, `smackdebt-project`,
  `smackdebt-output`, and the CLI composition root.
- Changes documented terminal output and JSON version 2 before the first public
  release.
- Does not add runtime tracing, remote services, persistent storage, or author
  names to reports.

