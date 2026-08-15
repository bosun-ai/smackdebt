## Why

The 2026-08-15 field runs on smackdebt, swiftide, and fluyt showed that the
strongest debt signal is computed in halves and never crossed, and that several
computed signals reach no reader.

- Churn and complexity are both measured, but never combined. A file that is
  both hot and complex ranks no higher than a cold one, so the report cannot say
  where debt actually hurts.
- Smackdebt's own top two findings are `assert_index_integrity · test`
  functions, ranked above `analyze_diff` (production, cognitive 44). For "is
  this code shit?", production debt has to come first.
- Package instability and contributor concentration are computed and serialized
  and then shown nowhere. Instability only becomes meaningful now that
  `resolve-workspace-dependencies` populates the workspace package graph.
- `--history` narrows activity only. Churn, coupling, and concentration ignore
  the window, so a stated window does not describe the numbers under it.
- Nesting depth is already tracked internally and never surfaced; parameter
  count, file size, and container size are not measured at all, so obvious debt
  shapes are invisible.
- Files that nothing depends on are indistinguishable from connected files.

This change supersedes the detail-selection parts of the retired
`make-terminal-detail-relevant` change, which chose which existing facts to
print without adding the signals a reader actually needs.

## What Changes

- `--history` governs churn, coupling, and concentration as well as activity.
  History coverage exposes the window and the count of commits excluded by it.
- Hotspots become a first-class signal: a rated file crossed with its change
  activity, with a minimum touch threshold of 5.
- Finding rank gains a hot key after triggered signals, then a role class key so
  primary source outranks non-primary source at equal rating.
- Stable-dependency violations become Watch architecture findings, computed by
  integer cross-multiplication with a minimum of 2 references.
- Knowledge concentration becomes a Watch evolutionary finding at 10 or more
  commits with a single-contributor share of at least 90%, expressed as counts
  only, with no contributor identity.
- Maximum nesting depth and parameter count are collected for every supported
  language and exposed as measurements. They are not rated in this change;
  rating promotion waits for `adopt-report-schema-v4` so every rating stays
  explainable from serialized measurements.
- File and container size become rated signals against defined thresholds.
- Orphan files — supported primary files with no incoming dependency that are
  not entry files — become descriptive facts.
- New report tables are ordered by data-stable keys and no new traversal or Git
  process is introduced. Terminal sections, labels, and vocabulary are unchanged
  and terminal bytes move only where the new rank keys reorder findings. JSON
  version 3 shape is unchanged.

## Capabilities

### New Capabilities

- `hotspot-analysis`: Crosses rated source debt with change activity and owns
  the hot and role-class rank keys.

### Modified Capabilities

- `evolutionary-analysis`: Window-governed history signals, window coverage
  fields, and knowledge-concentration findings.
- `architecture-analysis`: Stable-dependency Watch findings and orphan file
  facts.
- `metric-semantics`: Maximum nesting, parameter count, and size definitions.
- `language-analysis`: Units expose the new collected measurements and every
  language has exact fixtures for them.
- `analysis-performance`: New signals stay inside the existing passes with
  data-stable ordering.
- `product-documentation`: The documented rank sequence matches the new keys.
- `end-to-end-evidence`: Generated fixtures prove every new signal.
- `release-readiness`: Keeps publication blocked through the five v-next
  changes.

## Dependency Order

The v-next set is implemented, reviewed, and archived in this order:

1. `resolve-workspace-dependencies`
2. `deepen-debt-signals` (this change)
3. `add-verdict-policy`
4. `redesign-terminal-report`
5. `adopt-report-schema-v4`

This change requires the workspace package graph produced by
`resolve-workspace-dependencies`; without it, stable-dependency findings have
almost no input. `prepare-first-release` stays blocked until all five are
archived.

## Impact

This changes analysis policy, finding rank, report tables, and the facts
available to later presentation and schema work. It affects health policy,
evolution, contributor concentration, the new hotspot, stable-dependency, and
size policies, language measurement collection, and configuration thresholds. It
does not change terminal sections or vocabulary, JSON version 3 shape, the verdict, or rating for
nesting and parameter count; those belong to the three later changes.
