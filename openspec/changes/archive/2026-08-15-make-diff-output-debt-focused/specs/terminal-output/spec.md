## MODIFIED Requirements

### Requirement: Default sections show only relevant decisions
Default codebase views SHALL retain the accepted relevance policy. Default diff
views SHALL render from the analysis-owned selection's three typed lists. After
the normal two-line heading and breadcrumb, a diff whose source, architecture,
and evolution lists are empty and which has no real changed-source gap SHALL print the
sole result line `No debt changed.`. It SHALL omit `QUALITY`, `AREAS`,
`FINDINGS`, `ARCHITECTURE`, `HISTORY`, warnings, and discover guidance.

When real changed-source gaps exist but all three lists are empty, the sole result
line SHALL instead be `No debt changed in checked files.` followed by one
grouped changed-source warning. It SHALL omit all debt sections and discover
guidance. Otherwise diff `QUALITY` SHALL show only nonzero trusted source debt
counts in Worse, Better, Changed glyph order. Source `AREAS` SHALL show at most
five children with source debt, and source discover SHALL point only to the
first displayed source-debt child. Source detail SHALL show at most three linked
comparisons in stable direction and rank order. `ARCHITECTURE` and `HISTORY`
SHALL each independently show at most three introduced or removed findings and
SHALL NOT contribute to `DebtDiffCounts`, `QUALITY`, source `AREAS`, or source
discover. Architecture-only and history-only results SHALL omit `QUALITY`,
`AREAS`, `FINDINGS`, and source discover and show only their applicable section
and selected warning. A mixed result SHALL keep the families separate. Every
empty optional section SHALL be absent.

#### Scenario: Diff has no human debt or warning
- **WHEN** retained comparisons contain only unchanged or healthy-only facts
- **THEN** output after heading and breadcrumb is exactly `No debt changed.` with no debt section, warning, or discover line

#### Scenario: Only changed-source gaps remain
- **WHEN** all three selection lists are empty and selected changed files include real incomplete analysis
- **THEN** output says `No debt changed in checked files.` followed by the exact singular/plural Warning-glyph changed-file row and no debt section or discover line

#### Scenario: Diff has several human debt families
- **WHEN** linked source, architecture, and evolution debt changes exist
- **THEN** `QUALITY` shows nonzero Worse, Better, Changed counts in that order and default sections apply their exact limits without empty headings

#### Scenario: Architecture-only or history-only debt exists
- **WHEN** the source list is empty and one non-source finding list is nonempty
- **THEN** its separate section appears without `QUALITY`, `AREAS`, `FINDINGS`, source discover, or a combined count

### Requirement: Detailed and path views remain useful
For codebase output, `--all` and path selection SHALL retain the accepted useful
debt policy. For diff output, `--all` SHALL remove limits only for child areas
with trusted source debt and trusted source, introduced/removed architecture, and
introduced/removed evolution IDs plus selected real diagnostics. A recovered
Watch/High comparison and every fixture/generated comparison SHALL remain
JSON-only in diff default, diff `--all`, and diff path even when it meets the
numeric relevance matrix. Trusted test, example, and benchmark comparisons
SHALL remain eligible. Excluded comparisons SHALL NOT change
`DebtDiffSelection`, `DebtDiffCounts`, `QUALITY`, `AREAS`, discover guidance, or
the no-debt decision. This diff-specific rule narrows the earlier general
codebase role/advisory detail policy because those roles do not enter trusted
diff verdict math and recovered before/after matching is not trusted. Codebase
default, path, and `--all` behavior SHALL remain unchanged.
`--all`
SHALL NOT show unchanged, ambiguous, healthy-only, ordinary relation, unchanged
history, activity, concentration, processing, or other contextual rows.

A selected diff path SHALL change scope only and SHALL apply the same default
policy unless `--all` is also supplied. Terminal output SHALL read the completed
private nonserialized per-scope typed lists and source-only `DebtDiffCounts`, visit each ID
once, and perform no trust
filtering, reclassification, count change, global comparison scan, identity map,
second selected-ID collection, or silent repair.

#### Scenario: User requests all diff debt
- **WHEN** `--all` is supplied for any diff scope
- **THEN** every trusted primary/test/example/benchmark human debt row appears while fixture/generated/recovered comparisons and excluded complete-report context remain absent

#### Scenario: Excluded-role comparison meets the numeric matrix
- **WHEN** fixture or generated source retains a Watch/High diff comparison
- **THEN** diff default, diff `--all`, and diff path say exactly `No debt changed.` with no optional detail while JSON retains role, coverage, measurements, comparison, and diagnostics unchanged

#### Scenario: Recovered comparison is the only debt-shaped fact
- **WHEN** a recovered Watch or High comparison meets the relevance matrix
- **THEN** diff default, diff `--all`, and diff path show exactly `No debt changed in checked files.` plus one grouped warning with no optional detail, while JSON retains the comparison

#### Scenario: User selects a diff path
- **WHEN** a package, directory, or file path is supplied without `--all`
- **THEN** only scope changes and default debt limits still apply

### Requirement: Human diagnostics are grouped and simple
Codebase diagnostics SHALL retain the accepted source and architecture warning
policy. Diff analysis SHALL aggregate unique selected changed file IDs once
across read failure, failed parse, recovered parse, and ambiguous unit match.
Human output SHALL render exactly `<Warning glyph> 1 changed file could not be
checked.` or `<Warning glyph> <count> changed files could not be checked.`. It
SHALL appear once for the selected scope, including immediately after `No debt
changed in checked files.`, and SHALL NOT expose raw reasons, per-unit rows, or
an ambiguous comparison card. JSON SHALL retain every exact diagnostic fact.
Architecture
resolution remains a separate grouped warning only when selected changed
relationship facts create it and human debt output is also present.
Architecture warnings, current-history warnings, and context unrelated to
selected changed facts SHALL NOT appear in a diff. A no-debt result SHALL allow
only the grouped changed-source warning variant; it SHALL NOT include an
architecture or history warning. No warning SHALL expose command, status,
parser, process, or other implementation text.

#### Scenario: Several changed files cannot be analyzed or matched safely
- **WHEN** selected changed files retain read, failed/recovered parse, or ambiguous-unit gaps, including several reasons on one file
- **THEN** each unique file counts once and the exact singular/plural Warning-glyph row appears without raw reasons or per-unit rows

#### Scenario: Selected changed relationships have incomplete resolution
- **WHEN** human debt output is present and selected changed relationship facts produce an architecture warning
- **THEN** that grouped architecture warning appears once without unrelated current architecture or history context

#### Scenario: No human debt exists with unrelated warnings
- **WHEN** current architecture or history context has a warning unrelated to selected changed facts
- **THEN** the exact no-debt state omits it, allowing only the grouped changed-source warning variant when applicable

### Requirement: Machine and analysis interfaces do not change
The system SHALL preserve every retained comparison, complete comparison link,
comparison kind and direction, complete scope count, report and analysis fact,
JSON version-3 field and exact byte, schema, index, privacy rule, CLI flag, exit
behavior, serial and automatic behavior, live work count, allocation, and
measured resource behavior. Human diff selection SHALL add only private
nonserialized analysis-owned `DebtDiffSelection` objects with unique trusted
source, architecture, and evolution ID lists plus source-only
`DebtDiffCounts`. Index audits SHALL reject a repeated typed ID within any one
list.

#### Scenario: Human diff selection becomes debt-focused
- **WHEN** the same fixture renders as default terminal, `--all`, path, and JSON
- **THEN** only intended terminal bytes differ while complete machine and analysis proof remains exact

#### Scenario: One selection repeats an ID
- **WHEN** report/index integrity checks trusted source, architecture, or evolution IDs
- **THEN** validation fails before terminal output can hide or rewrite it
