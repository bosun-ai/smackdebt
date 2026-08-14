## MODIFIED Requirements

### Requirement: Default terminal detail stays concise
Default codebase repository, package, directory, and file views SHALL retain the
accepted relevance policy. Default diff views SHALL apply the same scope levels
but use the trusted human-debt selection: at most five children with source
debt, three source comparisons, three introduced or removed architecture findings, three
introduced or removed evolution findings, selected grouped warnings, and one
source discover command. Source `QUALITY`, `AREAS`, and discover SHALL use only
source debt IDs. Architecture/history SHALL remain independent sections and
never enter source counts/navigation. Empty optional sections SHALL be absent. A selected path
SHALL change scope only.

`--all` SHALL remove limits only for the applicable codebase useful-debt links
or diff trusted human-debt selection and retain real selected diagnostics.
Fixture, generated, and recovered comparisons SHALL remain JSON-only in diff
default, diff `--all`, and diff path. Test, example, and benchmark comparisons
SHALL remain eligible. This diff-only narrowing SHALL NOT change codebase
default, path, or `--all`. Diff human output SHALL NOT show
healthy, unchanged, ambiguous-card, raw relationship, weak/explained history,
activity, concentration, coverage-field, processing, or other contextual rows.

#### Scenario: Default diff has many human debt links
- **WHEN** every human debt family exceeds its default limit
- **THEN** the selected scope shows five child areas and three rows in each source, architecture, and history detail group

#### Scenario: Only architecture or history debt exists
- **WHEN** the source debt list is empty and one non-source list is nonempty
- **THEN** its separate section appears without `QUALITY`, `AREAS`, `FINDINGS`, source discover, or a combined count

#### Scenario: Diff all mode is requested
- **WHEN** `--all` is supplied
- **THEN** every trusted selected ID appears without fixture/generated/recovered comparisons or excluded machine context

#### Scenario: Optional diff section selects no row
- **WHEN** an area, detail family, warning, or discover command is not relevant
- **THEN** its heading, table, placeholder, and command are absent

### Requirement: Diff comparisons have one displayed direction
Every retained comparison SHALL keep its existing complete direction and JSON
facts. Human source debt links SHALL include Regressed and Added with an after
rating of Watch or High as Worse, Improved and Removed with a before rating of
Watch or High as Better, and MetricChanged as Changed only when either side is
Watch or High. Unchanged, Ambiguous, and
healthy-only Added, Removed, or MetricChanged SHALL remain report and JSON facts
without a human displayed direction row. Ambiguous matching SHALL contribute to
the grouped changed-source warning.
Selected IDs SHALL exclude fixture, generated, and recovered comparisons in
diff default, diff `--all`, and diff path, even when their numeric facts meet the
same matrix. Fixture/generated roles do not enter trusted diff verdict math, and
recovered before/after matching is not trusted. The facts remain JSON-only;
selected recovered files contribute to the grouped changed-file warning.
Terminal SHALL apply no trust filter or reclassification.

#### Scenario: Watch unit is added
- **WHEN** a comparison is Added with an after rating of Watch
- **THEN** it contributes once to human Worse for its file and applicable ancestor scopes

#### Scenario: Healthy unit changes measurements
- **WHEN** MetricChanged is Healthy on both sides
- **THEN** its complete Changed fact remains in JSON and it has no human debt link

#### Scenario: Unit cannot be matched safely
- **WHEN** a comparison is Ambiguous
- **THEN** it remains complete in JSON and its selected changed file contributes once to the exact grouped changed-file warning in human output

### Requirement: Diff scopes expose change distribution
Each diff scope SHALL retain exact complete Worse, Better, and Changed counts and
all underlying report and JSON distribution facts. Analysis/report aggregation
SHALL also own one private nonserialized `DebtDiffSelection` with three unique
typed ID lists: trusted source debt comparisons, introduced/removed architecture
findings, and introduced/removed evolution findings. `DebtDiffCounts` SHALL
contain meaningful exact Worse, Better, and Changed values for trusted source
IDs only. Architecture and evolution IDs SHALL NOT contribute. Human `QUALITY`,
source area rows, and source discover SHALL use `DebtDiffCounts`, show only
nonzero glyph counts in Worse, Better, Changed order, and show no rates, shares,
percentages, or bars.

#### Scenario: Complete and human counts differ
- **WHEN** a scope contains healthy-only Changed comparisons and one Regressed debt comparison
- **THEN** JSON retains every complete count while terminal human counts include only the one Worse debt link

#### Scenario: Source and non-source debt are mixed
- **WHEN** source, architecture, and evolution lists all contain IDs
- **THEN** `DebtDiffCounts`, `QUALITY`, source `AREAS`, and source discover reflect only source IDs while architecture/history render separately

#### Scenario: Recovered comparison meets the numeric relevance matrix
- **WHEN** a recovered Watch or High comparison is present
- **THEN** diff default, diff `--all`, and diff path omit it, `DebtDiffCounts` and navigation remain unchanged, the grouped real warning remains, and JSON retains the comparison

#### Scenario: Fixture or generated comparison meets the numeric matrix
- **WHEN** default, `--all`, or path diff output is selected
- **THEN** completed selection omits it without renderer role filtering and JSON retains role, measurements, comparison, coverage, and diagnostics

#### Scenario: Human counts are all zero
- **WHEN** all three typed lists are empty
- **THEN** terminal says only `No debt changed.` after heading and breadcrumb, or `No debt changed in checked files.` followed only by the exact singular/plural Warning-glyph changed-file row when real gaps exist, and omits count rows and debt sections

### Requirement: Diff child and detail order is deterministic
Complete diff facts SHALL retain their existing stable order. Human child rows
SHALL sort by human Worse descending, Better descending, Changed descending,
then repository-relative path ascending. Human detail SHALL group Worse before
Better before Changed, then use existing stable finding rank, path, source span,
and typed ID tie-breakers. Default SHALL show at most five debt-bearing children
and the first three source comparisons plus the first three architecture and
evolution finding changes. `--all` SHALL preserve the same order without those
limits.

#### Scenario: Package IDs conflict with debt rank
- **WHEN** several linked comparisons have conflicting insertion, path, and rank order
- **THEN** terminal follows direction, rank, path, span, and typed ID order without scanning the complete table

### Requirement: Diff details state only meaningful changes
Human Regressed, Improved, and relevant MetricChanged cards SHALL show every
rated measurement whose numeric value changed, including mixed-direction
changes, each as before to after, plus
the exact retained comparison source location and line. Human Added cards SHALL
show every after-side measurement whose individual signal is Watch or High,
plus exact after location and line, and SHALL omit every Healthy measurement.
Removed cards SHALL show every before-side measurement whose individual signal
is Watch or High plus exact before location and line and SHALL omit every
Healthy measurement.
When location identity differs, presentation SHALL follow the report's existing
stable comparison-location policy. JSON SHALL retain both sides and all facts.
A displayed comparison SHALL NOT state only added, removed, or changed without
the relevance measurements and exact location/line required by its kind.

An unnamed unit SHALL use `<container> · closure`, `<container> · lambda`, or
`<container> · <kind>` with the nearest named container for human identity.
Without a named container it SHALL use `<filename> · <kind>`. A Vue template
SHALL use `<filename> · template`. Its exact `path:line` SHALL appear on the next
line and distinguish repeated anonymous units. A named unit SHALL retain its
real, possibly language-qualified name. Human output SHALL NOT use an
angle-bracket identity placeholder or anonymous `::kind` form. Content-aware layout SHALL preserve every required
measurement, location, line, direction, and identity without normal safety
fallback shortening.

#### Scenario: Regression changes one measurement
- **WHEN** a Regressed comparison changes only cognitive complexity
- **THEN** its card shows only cognitive before to after plus the retained comparison location and line

#### Scenario: Added Watch unit has one triggering signal
- **WHEN** only one after-side measurement makes an Added unit Watch
- **THEN** its card shows only that triggering measurement plus exact after location and line

#### Scenario: Added or removed unit has mixed metric signals
- **WHEN** its existing side has more than one Watch/High measurement and at least one Healthy measurement
- **THEN** every Watch/High measurement appears and every Healthy measurement is absent

#### Scenario: Relevant comparison has mixed numeric directions
- **WHEN** one measurement rises and another falls numerically
- **THEN** every changed measurement appears as before to after

#### Scenario: A direction label has no evidence
- **WHEN** a proposed human card says only added, removed, or changed
- **THEN** acceptance rejects it because the required relevance measurements and exact location/line are absent

#### Scenario: Anonymous Vue template changes
- **WHEN** a Vue template comparison has no source name or named container
- **THEN** human identity is `<filename> · template`, exact `path:line` follows, and JSON identity remains unchanged

#### Scenario: Repeated anonymous units share a kind
- **WHEN** two anonymous units in one file would otherwise display the same identity
- **THEN** their exact next-line `path:line` values keep them distinct and `<closure 1177>`, `<template>`, and anonymous `::kind` forms do not appear
