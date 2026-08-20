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
Each non-file codebase scope SHALL expose exact High and Watch counts for its children.
Terminal area rows SHALL state those counts in words and SHALL NOT add a share
percentage, an attention rate, or a rate bar, because a bar used as data and a
ratio the reader cannot check were removed from human output. A machine
consumer SHALL derive any share from the exact counts the report already
serializes.

#### Scenario: Debt is spread across child directories
- **WHEN** the selected scope contains several child areas with rated units
- **THEN** each debt-bearing row shows its exact High and Watch counts labeled by their words and no share, rate, or bar

#### Scenario: Selected scope has no debt
- **WHEN** the selected scope has zero High and zero Watch units
- **THEN** no area section is written

### Requirement: Codebase child order is severity-led
Codebase child area rows SHALL sort by High count descending, Watch count
descending, then repository-relative name ascending, so the child holding the
worst debt is read first and equal children keep a stable name order. Codebase
finding rows SHALL use the finding rank owned by `hotspot-analysis` rather than
any order of their own, and this specification SHALL NOT restate that rank's
keys. This supersedes the earlier text, which enumerated a finding rank of
rating, signals at that rating, total triggered signals, metrics, and recent
activity; that copy omitted role class and hot state, contradicted the accepted
rank once role and heat moved ahead of both signal counts, and described neither
the child rows nor the finding rows as implemented.

#### Scenario: Two child areas hold different debt
- **WHEN** one displayed child area has more High units than another
- **THEN** it appears first even when the other has more Watch units

#### Scenario: Two child areas hold equal debt
- **WHEN** two displayed child areas have equal High and Watch counts
- **THEN** their repository-relative names produce stable order

#### Scenario: Hot production debt meets colder debt in one scope
- **WHEN** the selected scope holds findings that differ in role class or hot state
- **THEN** the displayed finding order is exactly the finding rank `hotspot-analysis` accepts, so primary source precedes non-primary source at equal rating and hot state decides before either signal count

#### Scenario: Every rank key ties
- **WHEN** two findings tie on every key the finding rank compares before path
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
The default terminal view SHALL open with the verdict block, SHALL show `AREAS` only when
several debt-bearing child areas exist, and SHALL show at most five such rows.
It SHALL show the first three ranked findings or comparisons within the selected
scope. Zero-value optional facts and empty optional sections SHALL be omitted,
except that a verdict count SHALL always be printed with its word even when it
is zero. When a diff moves no debt, the view SHALL be the verdict line only.

`--top N` SHALL provide a middle level of detail between the default and
`--all`: it SHALL raise or lower only the number of displayed ranked findings
or comparisons to `N`, leaving the architecture and history section limits
unchanged. `N` SHALL be a positive integer, and `--top` SHALL conflict with
`--json` and with `--all`.

`--all` SHALL show all useful debt without count limits and SHALL NOT show raw
dependency edges, standard-library externals, churn dumps, cyclomatic-1 rows,
weak coupling, or healthy rows; JSON remains the complete view of those facts.

#### Scenario: Selected scope has more than five affected children
- **WHEN** the default terminal report omits child rows
- **THEN** it shows the five most relevant rows without omitted-row bookkeeping

#### Scenario: User requests complete useful terminal detail
- **WHEN** the user supplies `--all`
- **THEN** terminal output shows all useful debt rows, findings, comparisons, and debt-bearing relationships without raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, or healthy rows

#### Scenario: Optional section has no finding
- **WHEN** architecture or history has no actionable finding
- **THEN** the terminal omits that section

#### Scenario: A diff moves no debt
- **WHEN** no comparison or finding counts as debt movement
- **THEN** the view is the verdict line only, with no trailing history, warning, or context section

#### Scenario: A middle finding limit is requested
- **WHEN** the user supplies `--top 10` and twelve ranked findings exist
- **THEN** ten findings are shown while architecture and history sections keep their default limits

#### Scenario: The limit exceeds the findings
- **WHEN** the user supplies `--top 10` and four ranked findings exist
- **THEN** all four are shown and no filler or bookkeeping row appears

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
The terminal discover line SHALL target the first displayed debt-bearing child after
display filtering and sorting, using its repository-relative path. The line
SHALL be the word `next:` followed by `smackdebt <path>`, decorated with U+F46B
before the word when decoration is enabled, so the line reads without its
glyph.

#### Scenario: A deeper debt-bearing child exists
- **WHEN** the selected codebase scope has a displayed debt-bearing child
- **THEN** the report prints `next: smackdebt <path>` for that first row

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
Each diff scope SHALL expose exact Worse, Better, and Changed counts. Terminal area rows
SHALL state those counts in words and SHALL NOT add a share percentage or a
share bar, for the same reason the codebase area rows do not. A machine
consumer SHALL derive any share from the exact counts the report already
serializes.

#### Scenario: Changed units span several directories
- **WHEN** the selected diff scope has changes in several child areas
- **THEN** each row shows its exact direction counts labeled by their words and no share or bar

#### Scenario: Diff selection has no retained comparisons
- **WHEN** the selected scope moved no debt
- **THEN** no area section is written

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

### Requirement: Codebase summary states rated quality and coverage
The terminal SHALL open with a verdict block stating the selected scope, the analysis-owned
tier sentence, and the counts behind it with every count labeled by its word,
followed by the worst offender with its resolved path and reason when one
exists. When the verdict carries an unsupported-coverage qualifier, the
qualifier row SHALL render inside the verdict block directly under the tier
sentence, using the analysis-owned qualifier bytes. It SHALL omit healthy
counts, summary ratios, and decorative quality bars used as data. Coverage
gaps SHALL use one grouped warning sentence only when a gap exists.

#### Scenario: Selection is fully analyzed
- **WHEN** every selected source file is analyzed
- **THEN** the verdict block omits coverage-success text and no warning appears

#### Scenario: Some selected source is excluded
- **WHEN** unsupported or failed files exist
- **THEN** one grouped warning states the gap without treating those files as healthy

#### Scenario: A scope has nothing to check
- **WHEN** the selected scope has zero checked units
- **THEN** the verdict block states the `empty` tier sentence and its zero counts with their words

#### Scenario: The verdict is qualified
- **WHEN** the unsupported byte share exceeds the qualifier threshold
- **THEN** the qualifier row appears under the tier sentence inside the verdict block

### Requirement: Coverage notes describe source-analysis gaps
Coverage notes SHALL report selected source files that could not be analyzed.
Routine changed files that are not source candidates SHALL NOT appear as a
coverage problem.

#### Scenario: Diff contains documentation and configuration changes
- **WHEN** a worktree contains changed source and non-source files
- **THEN** the diff analyzes source changes without a coverage note for routine non-source changes

### Requirement: Terminal layout responds to available width
Terminal rendering SHALL choose each row's shape from that row's own content rather than
from report-level width tiers. A row SHALL stay aligned when its content fits
the resolved width and SHALL otherwise stack its facts on indented lines. Every
ANSI-stripped line SHALL have Unicode display width less than or equal to the
requested width. Measurements, exact counts, cycle witnesses, history commit
evidence, dependency state, commands, finding identity, and comparison identity
SHALL never be silently clipped, and a cycle witness SHALL never be shortened
with an ellipsis. Layout SHALL NOT add bars used as data, summary ratios,
healthy rows, or internal processing facts at any width.

#### Scenario: Wide terminal renders relevant rows
- **WHEN** the resolved width is 120 columns
- **THEN** rows that fit stay aligned and every fact remains visible

#### Scenario: Narrow terminal renders relevant rows
- **WHEN** the resolved width is 50 columns
- **THEN** rows that do not fit stack their facts on indented lines, no line exceeds 50 display cells or breaks a glyph, and no measurement, count, witness, evidence value, command, or identity is lost

#### Scenario: A cycle witness is long
- **WHEN** a cycle witness does not fit the resolved width
- **THEN** the witness stacks across lines and is never shortened with an ellipsis

### Requirement: Terminal styling is optional and semantic
Terminal styling SHALL use ANSI sequences only when the CLI resolves color as
enabled. It SHALL color only decorations: High and Worse red, Watch and Warning
ANSI-256 208 orange, Discover cyan, Better green, Changed normal, and the
verdict bar in its tier color.

Decoration SHALL be resolved beside color from terminal detection. Glyphs and
the tier bar SHALL NOT remain in redirected or undecorated output; every
severity, direction, and diagnostic SHALL be readable from its word alone, so
undecorated output SHALL contain no codepoint in U+E000–U+F8FF. This supersedes
the earlier rule that symbols remain in plain redirected output, which assumed
glyphs carried meaning that words did not.

#### Scenario: Styled and plain output are compared
- **WHEN** the same report, width, and detail choice are rendered with color on and off
- **THEN** removing ANSI sequences from styled output produces the plain output byte for byte and adjacent text is unstyled

#### Scenario: Output is redirected
- **WHEN** standard output is not a terminal
- **THEN** no ANSI sequence and no glyph appear and every meaning is stated in words

#### Scenario: JSON is requested
- **WHEN** a user selects JSON output
- **THEN** JSON contains no ANSI styling and no decoration, and the machine contract changes only through its own change

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
removed, and unsafe-to-match units SHALL keep their direct explanatory text, and
that text SHALL come first on the card. An added or removed unit has one side
only, so its card SHALL additionally state that present side's absolute
measurements — the after side for an added unit and the before side for a
removed unit — showing only measurements whose value is nonzero. When every
measurement of the present side is zero, the card SHALL be the direction word
alone. An unsafe-to-match card SHALL remain its explanatory sentence alone,
because no side of it can be trusted. These values SHALL be taken from the
comparison the report already carries, and the renderer SHALL derive nothing.

#### Scenario: One measurement changes
- **WHEN** a retained comparison changes only cognitive complexity
- **THEN** its terminal card shows the cognitive before and after values without repeating unchanged measurements

#### Scenario: A unit is added
- **WHEN** a comparison adds a unit whose after side has nonzero measurements
- **THEN** its terminal card states the direction word first and then the after-side absolute values of those nonzero measurements

#### Scenario: A unit is removed
- **WHEN** a comparison removes a unit whose before side has nonzero measurements
- **THEN** its terminal card states the direction word first and then the before-side absolute values of those nonzero measurements

#### Scenario: An added unit measures zero everywhere
- **WHEN** an added unit's after-side measurements are all zero
- **THEN** its terminal card is the direction word alone with no zero-valued facts

#### Scenario: Unit cannot be matched safely
- **WHEN** a retained comparison is ambiguous
- **THEN** its terminal card says that the identity could not be matched safely and states no measurement

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
exists and SHALL show rated cycle witnesses without arbitrary edge rows or edge
totals. Each shown witness SHALL state its severity in the word vocabulary,
`high` or `watch`, optionally decorated with its glyph when decoration is
enabled. This supersedes the earlier rule that witnesses appear without severity
words and with a status glyph. `--all` and path drill SHALL expose relevant
relationship detail.

#### Scenario: An acyclic graph has many edges
- **WHEN** default output is rendered
- **THEN** `ARCHITECTURE` and arbitrary edge totals are absent while detailed path output can still inspect relevant relationships

#### Scenario: A rated cycle exists
- **WHEN** default output is rendered
- **THEN** `ARCHITECTURE` shows the cycle witness once with its severity word, decorated only when decoration is enabled

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

### Requirement: Every selected scope has its own verdict
Analysis SHALL complete the root verdict while the report is built and SHALL
expose a pure function that produces the verdict for any other selected scope
from the completed report. A scope verdict SHALL use only that scope's counts,
findings, comparisons, and worst offender, so a package, directory, or file view
answers about that scope rather than the repository. Producing a scope verdict
SHALL perform no filesystem, Git, parser, or analysis work, and renderers SHALL
consume the completed verdict rather than deriving one.

#### Scenario: A user drills into a package
- **WHEN** a package scope is selected and its debt differs from the repository's
- **THEN** its verdict tier and counts describe that package

#### Scenario: A scope has no checked units
- **WHEN** a selected scope contains no rated units
- **THEN** its verdict tier is `empty`

#### Scenario: A scope verdict is requested for rendering
- **WHEN** a renderer needs the verdict for the selected scope
- **THEN** it reads completed facts and performs no analysis work
