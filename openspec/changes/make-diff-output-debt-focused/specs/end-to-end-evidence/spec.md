## MODIFIED Requirements

### Requirement: Terminal evidence is exact and stable
The system SHALL compare committed exact terminal bytes for dense repository,
package, directory, and file diff views in default and `--all` modes at widths
120, 100, 80, and 50. Public fixtures SHALL explicitly cover clean worktree;
docs/config-only; healthy Added; healthy Removed; retained Unchanged;
Ambiguous; High Added; Watch Added; High Removed; Watch Removed; Regressed;
Improved; same-rating Watch MetricChanged; same-rating High MetricChanged;
multi-signal Added and Removed with a Healthy metric; mixed-direction
Regressed, Improved, and relevant MetricChanged;
introduced cycle; removed cycle; introduced coupling; removed coupling;
Changed-only edge/context; and selected package, directory, and file paths.
Every applicable case SHALL cover default and `--all` at 120, 100, 80, and 50.
Source-only, architecture-only, history-only, and mixed fixtures SHALL prove
source `QUALITY`, `AREAS`, and discover use only source IDs; architecture/history
render separately with no combined count; and no-debt checks all three lists.
Fixtures SHALL also cover a recovered Watch/High comparison beside trusted debt
and a selected recovered changed-source fact as the only issue. They SHALL prove
the comparison is absent in diff default, diff `--all`, and diff path, never changes
selection/counts/navigation/no-debt decisions, and all three recovered-only modes are
exactly `No debt changed in checked files.` plus one grouped warning with no
optional detail. Exact JSON SHALL retain the recovered comparison unchanged.
Separate fixture-only and generated-only Watch/High comparison cases SHALL run
diff default, diff `--all`, and diff path at every reviewed width. Each human
result SHALL use exact `No debt changed.` output with no optional detail or
role-filtering work. Exact JSON SHALL retain role, coverage, measurements,
comparison, and diagnostics unchanged. Test, example, and benchmark cases SHALL
prove they remain trusted diff-verdict eligible.
Evidence SHALL also cover stable direction/rank, source-only nonzero `QUALITY`
counts and navigation, area/detail limits, every required measurement including
mixed numeric directions and mixed per-metric signals, exact locations,
named and anonymous identities, both no-debt states, grouped warnings, one
discover command, empty-section absence, glyph/color equality, Unicode width,
and zero normal safety-fallback shortening.

Reviewed default public repository diff and selected package, directory, and
file path results SHALL contain at most 60 nonempty lines at widths 120, 100,
and 80 and at most 100 at width 50. `--all` SHALL have no line budget and SHALL
retain every human debt row.

#### Scenario: Dense diff matrix renders
- **WHEN** every named scope and mode renders at every required width
- **THEN** exact snapshots preserve every required debt fact and omit every excluded context family

#### Scenario: No default human debt exists
- **WHEN** plain and changed-source-warning default fixtures render
- **THEN** exact output uses the required sole result sentence and only the warning variant appends one grouped warning

### Requirement: Public command evidence covers every revised decision
Exact black-box acceptance SHALL cover Regressed, Improved, debt-bearing
Added/Removed, relevant MetricChanged, excluded Unchanged/Ambiguous/healthy-only
comparisons, introduced/removed architecture and evolution findings, all default
limits, unlimited human-debt `--all`, path scope-only behavior, measurement and
location rules, anonymous container/filename/kind and Vue-template identities,
no empty sections, both no-debt states, selected warnings, and discover
guidance. A displayed comparison that says only added, removed, or changed
without its required relevance measurements and exact location/line SHALL fail.
Repeated anonymous-unit fixtures SHALL prove middle-dot identities plus exact
next-line `path:line` distinguish units, while named units retain their real
possibly language-qualified names. Text audits SHALL reject raw/context rows,
`<closure 1177>`, `<template>`, and anonymous `::kind` forms. Warning fixtures
SHALL aggregate unique selected changed files once across read failure,
failed/recovered parse, and ambiguous unit matching; assert exact singular and
plural `<Warning glyph> 1 changed file could not be checked.` and `<Warning
glyph> <count> changed files could not be checked.` rows; reject raw reason/per-unit rows; and prove
architecture resolution appears only
from selected changed relationship facts alongside debt output, unrelated
current architecture/history warnings and context remain absent, and no-debt
output permits only the grouped changed-source warning variant.
Selection proof SHALL inspect one private per-scope `DebtDiffSelection`, unique
typed source/architecture/evolution lists, and source-only `DebtDiffCounts`. It
SHALL prove default, `--all`, and path read completed lists without renderer trust
policy or reclassification, architecture/history never enter source counts, and
recovered comparisons remain outside every list.
Fixture and generated comparisons SHALL also remain outside every list, while
test, example, and benchmark comparisons that meet the matrix remain inside.

The same fixtures SHALL prove every comparison, field, direction, complete
count, exact JSON version-3 byte, schema, index, privacy rule, status, stream,
serial/automatic byte, analysis fact, rank, work counter, allocation, and
resource profile unchanged. Pure integrity proof SHALL reject a repeated
comparison ID in one selection. Instrumentation SHALL prove one visit per
selected ID and no trust filtering, reclassification, count
change, global scan, identity map, de-dup set, second selected collection,
project work, or large clone.

#### Scenario: Source and non-source selection families are compared
- **WHEN** source-only, architecture-only, history-only, and mixed fixtures render
- **THEN** source counts/navigation and independent finding sections remain separate with no combined count

#### Scenario: Human and machine diff evidence are compared
- **WHEN** terminal default, terminal `--all`, path, and JSON run from one fixture
- **THEN** human output follows its completed trusted selection while JSON retains exact complete bytes and all work proofs pass

#### Scenario: Fixture and generated comparisons are machine-only
- **WHEN** exact default, `--all`, and path diff flows contain only those comparisons
- **THEN** each human flow says exactly `No debt changed.` with no optional section and JSON retains every role, coverage, measurement, comparison, and diagnostic fact

#### Scenario: Repeated private selection ID is injected
- **WHEN** report/index integrity runs on the selection before output
- **THEN** it fails and no renderer cleanup structure hides the error

### Requirement: Three workload families have privacy-safe acceptance
After public proof passes, aggregate read-only review SHALL cover self, a private
mixed application, and a private Rust workspace. Each review SHALL record only
three separate outcomes: a clean diff stops at exact `No debt changed.` output;
a changed diff leads with debt direction plus its measurement/location
explanation; and selected default paths meet their line budgets. It SHALL also
record only whether `--all` includes all trusted human debt while recovered
comparisons remain JSON-only, path changes scope only,
excluded complete-report context is absent, warnings are selected-fact scoped,
JSON remains complete, and work/resources remain unchanged. Committed
evidence SHALL name only workload family and outcome categories and SHALL
contain no private name, path, source, identity, history, or raw terminal output.

#### Scenario: Clean diff is reviewed in each workload family
- **WHEN** aggregate-only clean-diff outcome is recorded
- **THEN** it confirms output stops at exact `No debt changed.` without raw private output

#### Scenario: Changed diff is reviewed in each workload family
- **WHEN** aggregate-only changed-diff outcome is recorded
- **THEN** it confirms the first debt result has direction plus measurement/location explanation without raw private output

#### Scenario: Selected default paths are reviewed in each workload family
- **WHEN** aggregate-only selected-path outcomes are recorded
- **THEN** they confirm the exact default line budgets without raw private output
