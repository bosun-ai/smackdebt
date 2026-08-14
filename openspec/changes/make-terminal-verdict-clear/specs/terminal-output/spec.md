## MODIFIED Requirements

### Requirement: Default sections show only relevant decisions
Default terminal output SHALL always show `QUALITY`. When checked count is
greater than zero, it SHALL have exactly two content lines. The first SHALL use
`<grouped-checked> checked · <nearest-tenth attention percent>% need attention`,
including grouped digits such as `1,686 checked`; the second SHALL use exact
High and Watch glyph counts. When checked count is greater than zero and
attention is zero, the first line SHALL retain `0.0% need attention`, the second
SHALL be `No findings.`, and empty glyph counts SHALL be absent. When checked
count is zero, `QUALITY` SHALL have the sole content line `Nothing was checked.`
and SHALL NOT show a percentage, findings sentence, or glyph count.

Attention percent SHALL be nonnegative and SHALL use only the existing High,
Watch, and rated-unit counts without adding a report field. Presentation SHALL
widen counts to an unsigned integer type that safely holds attention count
times 1000, then round from integer quotient and remainder. When twice the
remainder is greater than or equal to checked count, the tenths value SHALL
advance. One finding among 16 checked units SHALL therefore display `6.3%`.

`AREAS` SHALL appear only when several affected child areas exist, with at most
five rows. Codebase rows SHALL show exact High and Watch counts in stable
severity-led order. Diff rows SHALL show exact direction counts in their stable
order. Area rows SHALL NOT show local rate, debt share, change share,
percentages, or bars. The report and JSON SHALL retain all underlying facts.

`FINDINGS` SHALL show the first three ranked rows. `ARCHITECTURE` SHALL appear
only when an architecture finding exists and SHALL show cycle witnesses rather
than edge totals. `HISTORY` SHALL appear only when an actionable history finding
exists. The discover line SHALL contain only the Discover glyph and next
command. Default `HISTORY` SHALL show at most three actionable findings ordered
by shared commits descending, similarity descending, then stable package names
and IDs. Each history row SHALL retain its package identities, shared and union
commit counts, similarity, absent dependency state, and Watch glyph without a
repeated status word.

#### Scenario: Checked source needs attention
- **WHEN** default codebase output has High or Watch findings
- **THEN** `QUALITY` shows checked count and nearest-tenth attention percent on its first line and exact High and Watch glyph counts on its second line
- **AND** area rows show exact status counts in stable order without rates, shares, percentages, or bars

#### Scenario: Checked source has no findings
- **WHEN** checked count is greater than zero and attention count is zero
- **THEN** `QUALITY` shows the exact checked count with `0.0% need attention` followed by `No findings.` and no empty High or Watch glyph count

#### Scenario: Attention percent is exactly halfway between tenths
- **WHEN** 1 of 16 checked units needs attention
- **THEN** `QUALITY` displays `6.3% need attention`

#### Scenario: No units are checked
- **WHEN** checked count is zero
- **THEN** `QUALITY` displays only `Nothing was checked.` without a percentage, findings sentence, or glyph count

#### Scenario: Checked count uses grouped digits
- **WHEN** checked count is 1686
- **THEN** the first `QUALITY` line starts with `1,686 checked`

#### Scenario: Diff output has relevant findings
- **WHEN** Worse, Better, Changed, architecture, or history findings exist
- **THEN** the same section relevance and detail limits apply and diff area rows retain exact status counts without share display

### Requirement: Human output removes repeated and internal facts
Human terminal output SHALL omit repeated bars, area percentages and ratios,
healthy counts, repeated status words, arbitrary architecture edge totals, weak
history pairs, and history processing totals. The one-decimal `QUALITY`
attention percent and actionable history evidence SHALL remain because each
directly explains its verdict. Human output SHALL NOT use the phrases `complete
local stream`, `eligible mapping`, `retained units`, or `retained package
pairs`. Cognitive, cyclomatic, and statement values SHALL remain on applicable
source findings. The human terminal SHALL call the logical-line measurement
`statements`.

#### Scenario: A report contains source, graph, and history detail
- **WHEN** default output is rendered
- **THEN** each actionable fact appears once, `QUALITY` retains its one-decimal attention percent, and no area rate, share, bar, removed label, or processing fact appears

### Requirement: Human diagnostics are grouped and simple
A source coverage warning SHALL appear only for a selected source read failure,
failed parse, or recovered/advisory parse excluded from health. Fixture and
generated exclusions SHALL NOT be called a coverage gap. Primary, test,
example, and benchmark source SHALL remain part of the default verdict. Default
root, package, and directory views SHALL group real gaps by kind. `--all` SHALL
name every relevant failed or recovered file once. An explicitly selected
affected file SHALL name that file. No view SHALL repeat the summary sentence
for each fact. This change SHALL NOT alter default directory file-list detail;
that decision SHALL be deferred to `make-terminal-detail-relevant`.

Unmatched and ambiguous imports SHALL produce one grouped row, separate from
source coverage: `<Warning glyph> 1 import could not be followed.` or `<Warning
glyph> <count> imports could not be followed.` It SHALL NOT expose command,
status, parser, process, or other implementation text. Incomplete history and
rename gaps SHALL retain their short human sentences.

#### Scenario: Fixture or generated source is excluded by role
- **WHEN** selected fixture or generated source is excluded from the verdict as configured
- **THEN** terminal output does not report failed or incomplete coverage for that policy exclusion

#### Scenario: A verdict-affecting role is checked
- **WHEN** selected primary, test, example, or benchmark source is analyzed
- **THEN** its rated units affect the default verdict and are not treated as a coverage exclusion

#### Scenario: Several real gaps exist
- **WHEN** selected source has repeated read, parse, or recovered advisory gaps
- **THEN** default root, package, and directory views group the real gaps, `--all` names every relevant failed or recovered file once, and an explicitly selected affected file names that file

#### Scenario: Imports have uncertain resolution
- **WHEN** unmatched and ambiguous imports are both retained
- **THEN** one Warning-glyph row states the combined import count with exact singular or plural `could not be followed` wording, separate from source coverage

### Requirement: Human command text uses the same simple language
The system SHALL use short user-facing language for CLI help, argument errors,
runtime errors, terminal labels, and README examples and SHALL avoid internal
report, composition, stream, mapping, command, parser, and process terms. A
missing selected path SHALL exit with status 1, write empty stdout, and write
exact stderr bytes `smackdebt: path not found: <user-path>\n` from the original
user input without an absolute path or operating-system error code. A missing
Git ref SHALL exit with status 1, write empty stdout, and write exact stderr
bytes `smackdebt: Git ref not found: <ref>\n` without a Git command, process
status, or fatal output. `--all --json` SHALL exit with status 2, write empty
stdout, and write exact stderr bytes `smackdebt: --all cannot be used with
--json\n`. None of these failures SHALL append usage or help.

#### Scenario: Missing selected path is reported
- **WHEN** the user selects a path that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: path not found: <user-path>\n` with no usage or help tail

#### Scenario: Missing Git ref is reported
- **WHEN** the user selects a Git ref that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: Git ref not found: <ref>\n` with no usage or help tail

#### Scenario: All detail conflicts with JSON
- **WHEN** the user supplies `--all --json`
- **THEN** status is 2, stdout is empty, and stderr is exactly `smackdebt: --all cannot be used with --json\n` with no usage or help tail

### Requirement: Machine and analysis interfaces do not change
The system SHALL preserve JSON version 3 bytes, report facts, distribution
values, finding rank, analysis and role policy, exit classes, serial and
automatic behavior, live work counts, allocations, and measured resource
behavior unchanged through the terminal verdict change. Revised failures SHALL
preserve their existing stdout/stderr destinations and exit classes.

#### Scenario: Terminal verdict and errors change
- **WHEN** the same generated fixtures are rendered as JSON and terminal output and the revised failures are invoked
- **THEN** only intended human terminal or error bytes differ while machine, analysis, work, stream, exit-class, and resource checks still pass
