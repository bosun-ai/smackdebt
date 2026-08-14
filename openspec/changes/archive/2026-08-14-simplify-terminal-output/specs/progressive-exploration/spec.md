## MODIFIED Requirements

### Requirement: Default terminal detail stays concise
The default terminal view SHALL show `AREAS` only when several affected child
areas exist and SHALL show at most five such rows. It SHALL show the first three
ranked findings or comparisons within the selected scope. Zero-value optional
facts and empty optional sections SHALL be omitted.

#### Scenario: Selected scope has more than five affected children
- **WHEN** the default terminal report omits child rows
- **THEN** it shows the five most relevant rows without omitted-row bookkeeping

#### Scenario: User requests complete useful terminal detail
- **WHEN** the user supplies `--all`
- **THEN** terminal output shows all useful affected rows, findings, comparisons, and relevant relationships without healthy rows or internal processing facts

#### Scenario: Optional section has no finding
- **WHEN** architecture or history has no actionable finding
- **THEN** the terminal omits that section

### Requirement: Explore points to the next debt-bearing area
The terminal discover command SHALL target the first displayed debt-bearing
child after display filtering and sorting, using its repository-relative path.
The line SHALL contain only U+F46B and `smackdebt <path>`.

#### Scenario: A deeper debt-bearing child exists
- **WHEN** the selected codebase scope has a displayed debt-bearing child
- **THEN** the report prints one valid glyph-plus-command line for that first row

#### Scenario: No deeper debt-bearing child exists
- **WHEN** the selected scope is a file or all deeper children are healthy
- **THEN** the report omits the discover line

### Requirement: Codebase summary states rated quality and coverage
The terminal `QUALITY` section SHALL state rated units and the count needing
attention. It SHALL use exact High and Watch glyphs without severity words and
SHALL omit healthy counts, repeated summary ratios, and decorative quality bars.
Coverage gaps SHALL use a grouped Warning line only when a gap exists.

#### Scenario: Selection is fully analyzed
- **WHEN** every selected source file is analyzed
- **THEN** `QUALITY` omits healthy and coverage-success text

#### Scenario: Some selected source is excluded
- **WHEN** unsupported or failed files exist
- **THEN** one grouped warning states the gap without treating those files as healthy

### Requirement: Terminal layout responds to available width
Terminal rendering SHALL use aligned layouts appropriate to widths 120, 80,
and 50 while preserving the same relevant facts, exact glyphs, recognizable
source locations, and drill commands. Every ANSI-stripped line SHALL have
Unicode display width less than or equal to the requested width. Layout SHALL NOT add bars, summary ratios, healthy rows,
or internal processing facts at any width.

#### Scenario: Wide terminal renders relevant areas
- **WHEN** the resolved width is 120 columns
- **THEN** area rows and findings align without repeated status labels

#### Scenario: Medium terminal renders relevant areas
- **WHEN** the resolved width is 80 columns
- **THEN** the same facts remain aligned and readable

#### Scenario: Narrow terminal renders relevant areas
- **WHEN** the resolved width is 50 columns
- **THEN** changed metrics; relationship identities, counts, statuses, and evidence; closed cycle witnesses; finding identity, kind, role, location, and measurements; coupling identity, commit evidence, dependency state, and diff outcome; and activity identity, commit count, and churn use short indented lines
- **AND** each long identity shortens in the middle while every fact remains visible, no line exceeds 50 display cells or breaks a glyph, and the reviewed flows record zero unexpected-line safety shortenings

### Requirement: Terminal styling is optional and semantic
Terminal styling SHALL use ANSI sequences only when the CLI resolves color as
enabled. It SHALL color only status glyphs: High and Worse red, Watch and
Warning ANSI-256 208 orange, Discover cyan, Better green, and Changed normal.
Symbols SHALL remain in plain redirected output.

#### Scenario: Styled and plain output are compared
- **WHEN** the same report, width, and detail choice are rendered with color on and off
- **THEN** removing ANSI sequences from styled output produces the plain output byte for byte and adjacent text is unstyled

#### Scenario: JSON is requested
- **WHEN** a user selects JSON output
- **THEN** JSON contains no ANSI styling and its version-3 bytes remain unchanged

### Requirement: Reports expose evolution as a separate concern
The system SHALL retain history coverage, churn, coupling, and concentration in
the report and JSON. Default terminal output SHALL show `HISTORY` only for
actionable history findings and SHALL omit weak pairs and processing totals. It
SHALL show at most three findings ordered by shared commits descending,
similarity descending, then stable package names and IDs, each with shared
commits, union commits, similarity, and the absent code dependency.

#### Scenario: Default codebase output has actionable history
- **WHEN** an unexplained coupling finding exists
- **THEN** `HISTORY` shows that finding once with exact commit evidence and without a retained-pair or processing summary

#### Scenario: A user selects an evolution detail target
- **WHEN** a package is selected or `--all` is supplied
- **THEN** relevant actionable and contextual history facts are shown without healthy rows, weak default pairs, or internal processing facts

### Requirement: Architecture default shows witnesses rather than edge samples
Default `ARCHITECTURE` output SHALL appear only when an architecture finding
exists and SHALL show rated cycle witnesses without severity words, arbitrary
edge rows, or edge totals. `--all` and path drill SHALL expose relevant
relationship detail.

#### Scenario: An acyclic graph has many edges
- **WHEN** default output is rendered
- **THEN** `ARCHITECTURE` and arbitrary edge totals are absent while detailed path output can still inspect relevant relationships

#### Scenario: A rated cycle exists
- **WHEN** default output is rendered
- **THEN** `ARCHITECTURE` shows the cycle witness once with its status glyph
