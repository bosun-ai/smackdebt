## MODIFIED Requirements

### Requirement: Codebase scopes expose debt distribution
Each non-file codebase scope SHALL retain exact High and Watch counts, selected-
scope debt share, and local attention rate for its children in report and JSON
facts. Human terminal area rows SHALL show exact High and Watch counts in stable
severity-led order and SHALL NOT show debt share, local attention rate,
percentages, or bars.

#### Scenario: Debt is spread across child directories
- **WHEN** the selected scope contains several child areas with rated units
- **THEN** each human area row shows exact High and Watch counts in stable severity-led order without a rate, share, percentage, or bar
- **AND** report and JSON retain exact selected-scope debt share and local attention rate

### Requirement: Diff scopes expose change distribution
Each diff scope SHALL retain exact Worse, Better, and Changed counts and each
child's selected-scope change share in report and JSON facts. Human terminal
area rows SHALL show exact direction counts in stable order and SHALL NOT show
change share, percentages, or bars.

#### Scenario: Changed units span several directories
- **WHEN** the selected diff scope has changes in several child areas
- **THEN** each human area row shows exact Worse, Better, and Changed counts in stable order without a share, percentage, or bar
- **AND** report and JSON retain the exact selected-scope change distribution

### Requirement: Codebase summary states rated quality and coverage
The terminal `QUALITY` section SHALL derive its verdict from existing rated
facts without a new report field. When checked count is greater than zero, its
first line SHALL state `<grouped-checked> checked · <nearest-tenth attention
percent>% need attention`, including grouped digits such as `1,686 checked`.
Attention percent SHALL use widened unsigned integer multiplication by 1000 and
quotient/remainder rounding. It SHALL be nonnegative and SHALL advance the
tenths value when twice the remainder is greater than or equal to checked count,
so 1 of 16 SHALL display `6.3%`. Its second line SHALL state exact High and
Watch glyph counts, or `No findings.` when attention is zero. When checked count
is zero, `QUALITY` SHALL contain only `Nothing was checked.` with no percentage,
findings sentence, or glyph count. It SHALL omit
the old rated-unit wording, repeated need-attention count, duplicate icon total,
healthy counts, and decorative quality bars. Codebase area rows SHALL retain
exact High and Watch counts in stable severity-led order without local rates,
debt shares, percentages, or bars. Coverage warnings SHALL appear only for real
incomplete analysis.

#### Scenario: Selection has findings
- **WHEN** checked source contains High or Watch findings
- **THEN** `QUALITY` states checked count, one-decimal attention percent, and exact status counts in two lines
- **AND** ordered area rows use exact status counts without rates, shares, percentages, or bars

#### Scenario: Selection is fully analyzed with no findings
- **WHEN** checked count is greater than zero, every selected source file is analyzed, and attention count is zero
- **THEN** `QUALITY` states the checked count with `0.0% need attention`, then `No findings.`, without empty glyph counts or coverage-success text

#### Scenario: Attention share is an exact halfway value
- **WHEN** 1 of 16 checked units needs attention
- **THEN** the nearest-tenth terminal verdict displays `6.3% need attention`

#### Scenario: Selection has no checked units
- **WHEN** checked count is zero
- **THEN** the terminal verdict displays only `Nothing was checked.` without a percentage, findings sentence, or glyph count

#### Scenario: Checked count uses grouped digits
- **WHEN** checked count is 1686
- **THEN** the first terminal verdict line starts with `1,686 checked`

#### Scenario: Some selected source analysis is incomplete
- **WHEN** selected source has a read failure, failed parse, or recovered/advisory parse excluded from health
- **THEN** one grouped warning states the real gap without treating the affected source as healthy

### Requirement: Coverage notes describe source-analysis gaps
Coverage notes SHALL report only selected source read failures, failed parses,
and recovered/advisory parses excluded from health. Fixture and generated
exclusions SHALL NOT appear as failed or incomplete coverage. Primary, test,
example, and benchmark source SHALL remain part of the default verdict. Routine
changed files that are not source candidates SHALL NOT appear as a coverage
problem. Default root, package, and directory views SHALL group repeated real
gaps. `--all` SHALL name every relevant failed or recovered file once. An
explicitly selected affected file SHALL name that file. This change SHALL NOT
alter default directory file-list detail; that decision SHALL be deferred to
`make-terminal-detail-relevant`.

#### Scenario: Diff contains documentation and configuration changes
- **WHEN** a worktree contains changed source and non-source files
- **THEN** the diff analyzes source changes without a coverage note for routine non-source changes

#### Scenario: Fixture or generated source is excluded by policy
- **WHEN** role policy excludes fixture or generated source from the health verdict
- **THEN** the exclusion does not create a coverage warning

#### Scenario: A verdict-affecting role is selected
- **WHEN** primary, test, example, or benchmark source is analyzed
- **THEN** its rated units remain in the default verdict

#### Scenario: Parse evidence is incomplete
- **WHEN** a selected source parse fails or recovers as advisory evidence outside health
- **THEN** default root, package, and directory views group that real gap, `--all` names every relevant affected file once, and an explicitly selected affected file names that file

### Requirement: Terminal layout responds to available width
Terminal rendering SHALL choose compact or stacked row shape from measured
visible content rather than from fixed width tiers. Every ANSI-stripped line
SHALL have Unicode display width less than or equal to the requested width.
Paths and long identities SHALL shorten in the middle when required to fit while
retaining recognizable beginnings and endings. Measurements, exact counts, cycle closure, history
commit evidence, dependency state, commands, and finding or comparison identity
SHALL NOT be silently clipped. Layout SHALL NOT add area rates, shares,
percentages, bars, healthy rows, or internal processing facts at any width.

#### Scenario: Content fits on one row
- **WHEN** a complete row fits the resolved width
- **THEN** it uses the compact shape without a width-tier-only rewrite

#### Scenario: Content does not fit on one row
- **WHEN** a complete row exceeds the resolved width
- **THEN** related facts stack on indented lines and long paths or identities shorten in the middle without losing any measurement, count, evidence, state, command, or identity

#### Scenario: Reviewed widths are rendered
- **WHEN** default codebase, path, and diff flows render at widths 120, 100, 80, and 50
- **THEN** every visible line fits, all required facts remain, and test-only writer evidence records zero safety-fallback shortenings
- **AND** only a synthetic unexpected overflow test exercises the final safety fallback

### Requirement: Terminal counts and labels are easy to scan
Terminal rendering SHALL group large integer digits, use correct singular or
plural wording, display table zeroes as an en dash where a table remains, and
measure ANSI-stripped Unicode display width. Paths and long identities SHALL be
shortened in the middle within a computed budget when required to fit. Status counts, measurements,
cycle closure, history commit evidence, dependency state, commands, and the
recognizable beginning and end of each shortened identity SHALL remain visible.

#### Scenario: Area name exceeds available space
- **WHEN** an area row cannot fit its label and exact status counts
- **THEN** the label shortens in the middle or the row stacks while every exact count remains visible

#### Scenario: Finding or relationship identity exceeds available space
- **WHEN** its compact row exceeds the resolved width
- **THEN** identity and facts use measured middle shortening and stacking without invoking the final safety fallback
