## Context

The first terminal change makes verdicts direct and preserves facts at narrow
widths. The second applies one useful-debt detail policy to codebase scopes.
Diff presentation still follows complete comparison tables, so JSON context is
mistaken for a human debt decision. This change adds a private presentation
index produced by analysis/report aggregation and consumed directly by output.

## Goals / Non-Goals

**Goals:**

- Make every human diff row explain debt that worsened, improved, or changed in
  a still debt-bearing unit.
- Keep default diff concise at repository, package, directory, and file scope.
- Make `--all` mean all trusted human debt comparisons, not every retained
  comparison. Recovered comparisons remain JSON-only.
- Preserve exact measurements, meaningful locations, identities, warnings, and
  content-aware width behavior.
- Keep every comparison and exact JSON version-3 bytes unchanged.
- Render each completed private selection once without trust policy,
  reclassification, or scans.

**Non-Goals:**

- Changing source, architecture, or evolution analysis.
- Removing or rewriting retained comparisons, directions, counts, or JSON.
- Adding a CLI flag, JSON version, serialized field, or public Rust interface.
- Showing ordinary relationship changes or unchanged history context in human
  output.

## Decisions

### Analysis owns one trusted human-debt selection

Analysis/report aggregation creates one private nonserialized
`DebtDiffSelection` for each scope. It contains unique typed IDs for trusted,
default-eligible source debt comparisons in one list, introduced/removed
architecture-finding comparisons in a second list, and introduced/removed
evolution-finding comparisons in a third list. Its meaningful `DebtDiffCounts`
value object counts only trusted source IDs as Worse, Better, and Changed.
Architecture and history IDs never contribute to it. Recovered comparisons
never enter any list, even for `--all`. Index audits reject a repeated ID within
each typed list. Existing complete comparison links, directions, counts, report
facts, and JSON remain unchanged.

The terminal reads the completed selection for default and `--all`, applying
only the relevant count limit and visiting each ID once in stable direction and
rank order. It never applies trust policy or
reclassification, changes `DebtDiffCounts`, scans global comparison tables,
builds an identity map or second selected collection, or silently repairs
repeated data. The same comparison may be linked once in separate ancestor
scopes; within one scope selection it occurs once.

### Human source relevance is exact

The selection includes trusted, verdict-eligible source comparisons with kinds `Regressed`,
`Improved`, debt-bearing `Added`, debt-bearing `Removed`, and `MetricChanged`
when either side is Watch or High. They exclude `Unchanged`, `Ambiguous`, and
healthy-only added, removed, or metric movement. Ambiguous matching contributes
to one grouped changed-source warning rather than a finding row.

Fixture and generated comparisons are excluded from trusted diff verdict math,
so they remain JSON-only in diff default, diff `--all`, and diff path. Recovered
Watch/High comparisons meeting the numeric matrix are also not trusted human
diff verdicts and remain JSON-only in all three diff views. None affects the
selection, `DebtDiffCounts`, `QUALITY`, `AREAS`, discover guidance, or the
no-debt decision. This diff-specific policy narrows the earlier general
codebase/path/`--all` role and advisory visibility rules; codebase behavior stays
unchanged. Before/after matching is not trusted for recovered facts. A
selected recovered changed-source fact remains a real grouped warning in both
human modes; when it is the only issue, both are exactly `No debt changed in
checked files.` followed by the grouped warning.

Worse contains Regressed and debt-bearing Added. Better contains Improved and
debt-bearing Removed. Changed contains only relevant MetricChanged. Direction
order is Worse, Better, Changed, followed by existing stable finding rank,
path, span, and typed ID tie-breakers.

### Architecture and history show finding movement only

Human architecture and evolution diff links contain introduced findings as
Worse and removed findings as Better. Unchanged findings, ordinary relation
changes, historical activity, and other Changed context remain in report and
JSON only. Architecture rows keep closed witnesses; history rows keep shared
commits, union commits, similarity, and `not linked in code` where applicable.

### Source verdict and independent finding sections stay separate

Default `QUALITY` lists only nonzero source `DebtDiffCounts` rows, ordered Worse,
Better, Changed and using the accepted status glyphs. Source `AREAS` appears
only for several children with source debt and shows at most five in stable
source direction order. Source discover guidance points only to the first
displayed source-debt child. Source `FINDINGS` shows the first three source IDs.
`ARCHITECTURE` and `HISTORY` independently show at most three IDs from their own
lists and never alter source counts, areas, or discover guidance.

A source-only result shows source summary/detail without empty architecture or
history sections. An architecture-only result omits `QUALITY`, `AREAS`,
`FINDINGS`, and source discover guidance and shows `ARCHITECTURE`. A history-only
result has the same omissions and shows `HISTORY`. A mixed result preserves the
same separation and never combines architecture/history into source counts.
Selected real source warnings, the grouped architecture warning, and at most
one source discover command remain when relevant.

`--all` removes limits only for source-debt areas and trusted human source,
architecture, and history IDs and retains selected real diagnostics. Recovered
fixture, generated, and recovered comparisons remain JSON-only. Its `QUALITY`, source area counts, and source
discover choice remain the `DebtDiffCounts` result. The no-debt decision checks
all three selection lists plus real changed-source gaps. A path changes
scope only. Neither mode enables unchanged, ambiguous, healthy-only, ordinary
relation, activity, concentration, or processing rows.

Warnings follow selected changed facts. Analysis aggregates unique selected
changed file IDs across read failure, failed parse, recovered parse, and
ambiguous unit matching. Each file counts once even if it has several reasons.
Human output shows exactly `<Warning glyph> 1 changed file could not be
checked.` or `<Warning glyph> <count> changed files could not be checked.` with
no raw reason or per-unit row. Architecture
resolution appears only when selected changed relationship facts produce it and
human debt output is present. Current architecture or history warnings and
context unrelated to selected changed facts never appear. The no-debt states
permit only the changed-source warning variant described below.

When all three selection lists are empty and there is no real changed-source
gap, output after the normal two-line heading and breadcrumb is the
sole result line `No debt changed.`. `QUALITY`, `AREAS`, `FINDINGS`,
`ARCHITECTURE`, `HISTORY`, warnings, and discover guidance are absent. When real
changed-source gaps exist but all three lists are empty, the result is `No debt
changed in checked files.` followed by one grouped real changed-source warning.
Debt sections and discover guidance remain absent.

### Cards retain useful evidence

Regressed, Improved, and relevant MetricChanged cards show every measurement
whose numeric value changed, including mixed-direction changes, each as before
to after, plus the exact retained
comparison source location and line. Added cards show only after-side triggering
measurements whose individual signal is Watch or High and no Healthy
measurement, plus the exact after location and line. Removed cards show every
before-side measurement whose individual signal is Watch or High and no Healthy
measurement, plus the exact before location and line. If location identity
differs, human output follows the report's existing stable comparison-location
policy while JSON retains both facts. Values, locations, and lines are never
silently clipped; content-aware layout stacks and middle-shortens identities or
paths as needed.

An unnamed unit uses `container · closure`, `container · lambda`, or
`container · kind` with its nearest named container. Without a named container
it uses `filename · kind`. A Vue template uses `filename · template`. Its exact
`path:line` appears on the next line and distinguishes repeated anonymous units.
Named units retain their real, possibly language-qualified name. Human output
never emits an angle-bracket placeholder or anonymous `::kind` form.
These identities are presentation-only and do not change JSON names,
containers, spans, or kinds.

### Proof covers complete and selected interfaces

Dense generated fixtures cover repository, package, directory, and file diff
views in default and `--all` modes at widths 120, 100, 80, and 50. Exact
snapshots cover every relevance kind, stable order, counts, no-debt states,
warnings, anonymous identities, measurements, locations, section absence,
commands, glyphs, colors, and zero safety-fallback shortening. Default public
codebase and selected-path results keep the accepted nonempty-line budgets;
`--all` has no line budget.

The public cases are explicit: clean worktree; docs/config-only change; healthy
Added; healthy Removed; retained Unchanged; Ambiguous; High Added; Watch Added;
High Removed; Watch Removed; Regressed; Improved; same-rating Watch
MetricChanged; same-rating High MetricChanged; introduced cycle; removed cycle;
introduced coupling; removed coupling; Changed-only edge/context; and selected
package, directory, and file paths. Each applicable case runs in default and
`--all` at widths 120, 100, 80, and 50.

Comparison-card assertions reject a row that says only added, removed, or
changed without its required relevance measurements and location/line. Identity
assertions reject literal `<closure 1177>` and `<template>`, and use repeated
anonymous units to prove exact source lines keep their displayed identities
distinct.

Fixture-only and generated-only Watch/High comparisons run through diff
default, diff `--all`, and diff path and produce exact `No debt changed.` output
with no optional section; JSON retains role, coverage, measurements,
comparison, and diagnostics. Recovered-only cases use the exact checked-files
warning result in all three diff views. Test, example, and benchmark cases prove
trusted diff-verdict participation. The same codebase fixtures prove their
previous default/path/`--all` behavior is unchanged.

The same fixtures prove exact JSON bytes, schema, indexes, comparisons,
directions, old complete counts, privacy, serial/automatic equality, and no
extra source, Git, parser, analysis, or worker work. Pure instrumentation proves
one link visit, no global scans, no second selected collection, no large clone,
and unchanged allocation/resource limits. Read-only self, mixed-application,
and Rust-workspace reviews record aggregate outcomes only. For each family, a
clean diff stops at exact `No debt changed.` output; a changed diff leads with
debt direction and its measurement/location explanation; and selected default
paths meet their line budgets. Review records contain no raw private output.
Default repository,
package, directory, and file diff results contain at most 60 nonempty lines at
widths 120, 100, and 80 and at most 100 at width 50. `--all` has no line budget.

## Risks / Trade-offs

- Human Changed becomes narrower while JSON retains complete change context.
- A diff can have retained changes but no human debt rows; the explicit no-debt
  state makes that distinction visible.
- Private presentation links add report memory; reserved construction and
  measured allocation/peak-memory checks keep the cost visible.

## Migration Plan

1. Strict-validate this authored change.
2. Wait for the first two terminal changes to be reviewed and archived.
3. Add pure relevance, direction, count, identity, integrity, and no-work tests.
4. Build private per-scope trusted selection objects during analysis/report aggregation.
5. Render diff sections directly from completed links and counts.
6. Update help, README, architecture guidance, and guarded exact snapshots.
7. Run unchanged-machine, resource, and aggregate workload proof.
8. Review and archive this change before release preparation resumes.

Rollback restores earlier human diff selection and snapshots. Serialized report
and JSON data require no migration.
