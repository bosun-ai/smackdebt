# progressive-exploration Specification

## Purpose
TBD - created by archiving change add-progressive-debt-exploration. Update Purpose after archive.
## Requirements
### Requirement: Reports identify an initial selected scope
Every completed codebase and diff report SHALL identify one initial selected
scope separately from the report facts. A renderer SHALL be able to render any
scope retained by that report without changing the report or running project
work again.

#### Scenario: Root report is explored several times
- **WHEN** a renderer selects the repository, a package, and a directory from one completed root report
- **THEN** every view is produced from retained report facts without filesystem, Git, parser, or worker activity

#### Scenario: Explicit path limits the report
- **WHEN** a user runs Smackdebt with an existing directory path
- **THEN** that directory is the initial selected scope and its shares use that scope's totals

#### Scenario: Selected directory has no supported source
- **WHEN** an existing selected directory contains no supported source files
- **THEN** Smackdebt produces a successful zero-total selected scope instead of replacing it with another scope

### Requirement: Codebase scopes expose debt distribution
Each non-file codebase scope SHALL expose exact High and Watch counts for its
children. Each displayed child SHALL expose both its share of selected-scope
debt and its local attention rate across all rated units in that child. Terminal
area views SHALL add a rate bar whose non-zero value remains visible.

#### Scenario: Debt is spread across child directories
- **WHEN** the selected scope contains several child areas with rated units
- **THEN** each debt-bearing row shows exact High and Watch counts, selected-scope debt share, local attention rate, and a rate bar

#### Scenario: Selected scope has no debt
- **WHEN** the selected scope has zero High and zero Watch units
- **THEN** the terminal states that no child areas need attention and shows no empty distribution table

#### Scenario: Small non-zero rate is rendered
- **WHEN** a child attention rate is greater than zero but smaller than one whole bar cell
- **THEN** the rate bar contains a fractional filled block

### Requirement: Codebase child order is severity-led
Default codebase rows SHALL use the exact finding rank: rating, count of signals
at that rating, total triggered signals, cognitive complexity, cyclomatic
complexity, logical lines, recent activity, then repository-relative path and
source span. Each comparison SHALL be descending except path and span, which
SHALL be ascending.

#### Scenario: Two findings share a rating
- **WHEN** one triggers more signals at that rating
- **THEN** it appears first even when the other has greater recent activity

#### Scenario: Every numeric key ties
- **WHEN** rating, signal counts, metrics, and activity are equal
- **THEN** path and span produce stable order

### Requirement: Structural single-child chains are passed visibly
Terminal rendering SHALL pass through repository, package, or directory scopes
that have exactly one child until it reaches a file or a scope with several
children. It SHALL show the passed repository-relative path as a breadcrumb.

#### Scenario: Repository contains one root package and one source directory
- **WHEN** the root package and source directory each have one child
- **THEN** the terminal view passes through both, prints the resulting breadcrumb, and avoids an empty `.` package row

#### Scenario: JSON consumer reads the same report
- **WHEN** terminal rendering passes through structural scopes
- **THEN** JSON still contains every original scope and parent-child link

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

### Requirement: Codebase detail follows existing hotspot priority
Every displayed finding SHALL show unit kind and SHALL show SourceRole whenever
the role is not primary. Recovered advisory findings SHALL use the same ranking
inside `--all` but SHALL remain absent from default detail.

#### Scenario: A benchmark function is High
- **WHEN** it appears in default detail
- **THEN** its unit kind and benchmark role are visible

#### Scenario: A recovered method is Watch
- **WHEN** detailed output is requested
- **THEN** its unit kind, role when non-primary, and advisory trust are visible

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

### Requirement: Diff comparisons have one displayed direction
Every retained comparison SHALL belong to exactly one of Worse, Better, or
Changed. `Regressed` and debt-bearing `Added` comparisons SHALL be Worse.
`Improved` and debt-bearing `Removed` comparisons SHALL be Better. All other
retained comparisons SHALL be Changed.

#### Scenario: Watch unit is added
- **WHEN** a comparison is `Added` with an after rating of Watch
- **THEN** it contributes once to Worse for its file and every ancestor scope

#### Scenario: High unit is removed
- **WHEN** a comparison is `Removed` with a before rating of High
- **THEN** it contributes once to Better for its file and every ancestor scope

#### Scenario: Healthy unit is added or removed
- **WHEN** an added or removed unit is Healthy on its existing side
- **THEN** it contributes once to Changed

#### Scenario: Measurements move within one rating
- **WHEN** a comparison is `MetricChanged`
- **THEN** it contributes once to Changed while retaining its detailed kind and measurements

### Requirement: Diff scopes expose change distribution
Each diff scope SHALL expose exact Worse, Better, and Changed counts. A child's
share SHALL equal its sum of those counts divided by the selected scope's sum.
Terminal area views SHALL visualize that share without replacing exact counts.

#### Scenario: Changed units span several directories
- **WHEN** the selected diff scope has changes in several child areas
- **THEN** each row shows exact direction counts, its nearest-whole-percent share of selected changed units, and a share bar

#### Scenario: Diff selection has no retained comparisons
- **WHEN** the selected scope has a zero change-count denominator
- **THEN** every child share is `0%` and its bar is empty

### Requirement: Diff child and detail order is deterministic
Diff child rows SHALL sort by Worse descending, Better descending, Changed
descending, then repository-relative path ascending. Comparison detail SHALL
group Worse before Better before Changed and use stable path and source order
within each direction.

#### Scenario: Several areas changed
- **WHEN** one child has more Worse outcomes than another
- **THEN** it appears first even when the other child has more total changes

#### Scenario: File diff is selected
- **WHEN** the selected file has several detailed comparison kinds
- **THEN** the terminal view shows every comparison in stable direction, path, and source order

### Requirement: JSON version 1 exposes progressive links additively
JSON schema version 1 SHALL add the initial selected scope, indexed report paths,
scope finding and comparison links, scope diff counts, comparison file ownership,
and comparison direction. Existing fields, types, and meanings SHALL remain
unchanged.

#### Scenario: Integration reads a codebase report
- **WHEN** JSON schema version 1 contains progressive fields
- **THEN** the integration can traverse from the selected scope to children and retained findings using indexes

#### Scenario: Integration reads a diff report
- **WHEN** JSON schema version 1 contains comparisons
- **THEN** each comparison identifies its file, detailed kind, derived direction, measurements, and rating transition

#### Scenario: Terminal output is truncated
- **WHEN** terminal row or detail limits apply
- **THEN** JSON still serializes the complete retained report

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

### Requirement: Coverage notes describe source-analysis gaps
Coverage notes SHALL report selected source files that could not be analyzed.
Routine changed files that are not source candidates SHALL NOT appear as a
coverage problem.

#### Scenario: Diff contains documentation and configuration changes
- **WHEN** a worktree contains changed source and non-source files
- **THEN** the diff analyzes source changes without a coverage note for routine non-source changes

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

### Requirement: Terminal counts and labels are easy to scan
Terminal rendering SHALL group large integer digits, use correct singular or
plural wording, display table zeroes as an en dash, and align using Unicode
display width. Only area labels may be shortened in the middle.

#### Scenario: Area name exceeds its column
- **WHEN** a wide or compact table cannot fit an area label
- **THEN** the label is shortened in the middle while its ending and all other columns remain visible

#### Scenario: Location exceeds terminal width
- **WHEN** a finding location is wider than the resolved terminal
- **THEN** the full location remains present and copyable

### Requirement: Color and width are CLI-owned choices
The CLI SHALL accept `--color auto|always|never`. Automatic mode SHALL enable
color only for a terminal when `NO_COLOR` is absent. Explicit color selection
SHALL conflict with `--json`. `COLUMNS` SHALL override width; otherwise a
connected terminal SHALL use its reported width and a redirected stream SHALL
use 100 columns.

#### Scenario: Output is redirected
- **WHEN** standard output is not a terminal and no width or color override is supplied
- **THEN** the terminal renderer receives width 100 and color disabled while retaining Unicode layout

#### Scenario: User forces color through a pipe
- **WHEN** standard output is redirected with `--color always`
- **THEN** ANSI styling is present

#### Scenario: User disables automatic color
- **WHEN** `NO_COLOR` is present or `--color never` is supplied
- **THEN** ANSI styling is absent

### Requirement: Package summary follows package scopes
Terminal output SHALL render package path `.` as `repository root` in headings,
rows, breadcrumbs, witnesses, and drill guidance. JSON and all machine indexes
SHALL retain `.` unchanged.

#### Scenario: The root is one package
- **WHEN** terminal and JSON render the same report
- **THEN** terminal says `repository root` and JSON package path remains `.`

### Requirement: Diff details state only meaningful changes
Terminal comparison cards SHALL state direction and identity, retain an
unbroken location, and show only measurements whose values changed. Added,
removed, and unsafe-to-match units SHALL use direct explanatory text.

#### Scenario: One measurement changes
- **WHEN** a retained comparison changes only cognitive complexity
- **THEN** its terminal card shows the cognitive before and after values without repeating unchanged measurements

#### Scenario: Unit cannot be matched safely
- **WHEN** a retained comparison is ambiguous
- **THEN** its terminal card says that the identity could not be matched safely

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

### Requirement: Default terminal sections do not duplicate evidence
A retained fact SHALL appear once in the nearest useful default terminal
section. Source, architecture, and evolution summaries SHALL NOT repeat an
identical finding, relation, coupling pair, history row, or operand already
shown as detail in the same view.

#### Scenario: A coupling finding is the leading evolution fact
- **WHEN** the default report includes its detail
- **THEN** another default section does not repeat the same pair and operands

### Requirement: Renderers consume completed presentation facts
Analysis SHALL own rank keys, verdict inclusion, advisory state, and stable
ordering. Terminal and JSON SHALL read one completed report without
reclassification, trust policy, filesystem, Git, parser, or analysis work.

#### Scenario: One report renders in two formats
- **WHEN** terminal and JSON output are selected in separate runs
- **THEN** role, trust, package, relation, history, and finding facts agree

