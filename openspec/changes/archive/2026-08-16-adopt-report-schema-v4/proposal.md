## Why

The machine contract still describes the product Smackdebt was, not the one the
four earlier v-next changes produce.

- Version 3 has no verdict and no summary. A machine consumer must join flat
  tables to answer the one question the tool exists to answer, and the 2026-08-15
  field runs show why that matters: 3.4MB of JSON on fluyt and 194KB on
  smackdebt, none of it leading with an answer.
- Hotspots, size findings, orphan files, stable-dependency findings, knowledge
  concentration, history window fields, and manifest names now exist in the
  report and have nowhere to go in version 3.
- Version 3 serializes `similarity` and `ratio` as floating-point values even
  though both are exact integer operands elsewhere in the same object, which
  contradicts the integer-only arithmetic rule.
- Maximum nesting depth and parameter count have been collected since
  `deepen-debt-signals` and are still unrated, because rating them was
  deliberately deferred to the change that also serializes them.
- The retired `report-schema-v2` capability still sits in accepted specs
  describing a contract the CLI does not present.

## What Changes

- `schema_version` becomes 4 and a checked `report-v4` schema replaces the
  version-3 schema.
- The JSON object opens with a denormalized head: `verdict` with `tier`,
  `sentence`, and `mode`, and `summary` with checked, high, and watch counts,
  the debt-diff counts, and up to three fully resolved worst entries carrying
  path strings — the common question answered with zero joins.
- New tables `hotspots`, `size_findings`, `orphan_files`,
  `stable_dependency_findings`, and `knowledge_concentration_findings` are
  added. Every finding family that owns an identity type in analysis owns its
  own table and states its `kind`, `history_coverage` gains the window fields,
  comparisons gain their nullable source location, and package records gain
  `manifest_name`.
- Serialized `similarity` and `ratio` floating-point values are removed; their
  integer operands remain, so every serialized value is an integer or a string.
- Maximum nesting depth and parameter count are promoted to rated signals with
  thresholds Watch 4 and High 7 for nesting and Watch 6 and High 9 for parameter
  count, amending the rule that a rating is explainable from three measurements.
- The `report-schema-v3` capability is removed and the stale `report-schema-v2`
  capability is removed with it.

## Capabilities

### New Capabilities

- `report-schema-v4`: The machine report contract, its denormalized head, its
  tables, and its executable schema.

### Removed Capabilities

- `report-schema-v3`: Replaced by version 4; the CLI no longer presents it.
- `report-schema-v2`: Stale since version 3 and never presented by the CLI.

### Modified Capabilities

- `metric-semantics`: Nesting and parameter count become rated signals with
  exact thresholds.
- `evolutionary-analysis`: Coupling similarity and concentration ratio are
  derived from operands and are no longer serialized.
- `language-analysis`: Ratings are explainable from five rated measurements.
- `product-documentation`: README documents version 4 and its head.
- `end-to-end-evidence`: JSON evidence validates version 4.
- `release-readiness`: Version-3 retirement is satisfied; publication stays
  blocked until all five v-next changes are archived.

## Dependency Order

The v-next set is implemented, reviewed, and archived in this order:

1. `resolve-workspace-dependencies`
2. `deepen-debt-signals`
3. `add-verdict-policy`
4. `redesign-terminal-report`
5. `adopt-report-schema-v4` (this change)

This change serializes what the four earlier changes produce, so it is last.
`prepare-first-release` resumes only after this change is archived, and its
version-3 retirement requirement is satisfied by version 4.

## Impact

This changes the machine contract, every JSON snapshot, the checked schema, and
the ratings of units that nest deeply or take many parameters. It affects report
construction, health policy, JSON serialization, configuration thresholds,
schema documentation, `ARCHITECTURE.md`, `AGENTS.md`, and the README. Terminal
sections and vocabulary from `redesign-terminal-report` are unchanged, though
terminal bytes move where a promoted rating changes a finding.
