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
problem rows SHALL use the problem rank owned by `problem-clustering` rather
than any order of their own, and this specification SHALL NOT restate that
rank's keys.

This replaces the earlier sentence that codebase finding rows use the finding
rank owned by `hotspot-analysis`: codebase debt rows are problem cards now, and
the accepted finding rank survives unchanged inside the problem rank as one of
its keys and inside worst-offender selection. Diff comparison rows keep their
own accepted order. The earlier text this supersedes also enumerated a finding
rank of rating, signals at that rating, total triggered signals, metrics, and
recent activity; that copy omitted role class and hot state, contradicted the
accepted rank once role and heat moved ahead of both signal counts, and
described neither the child rows nor the finding rows as implemented.

#### Scenario: Two child areas hold different debt
- **WHEN** one displayed child area has more High units than another
- **THEN** it appears first even when the other has more Watch units

#### Scenario: Two child areas hold equal debt
- **WHEN** two displayed child areas have equal High and Watch counts
- **THEN** their repository-relative names produce stable order

#### Scenario: Problems of differing severity share a scope
- **WHEN** the selected codebase scope holds cards that differ in rating, claimed High count, and hot state
- **THEN** the displayed order is exactly the problem rank `problem-clustering` accepts, and no renderer reorders it

#### Scenario: Every rank key ties
- **WHEN** two cards tie on every key the problem rank compares before the anchor
- **THEN** anchor path and anchor start line produce stable order

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
Zero-value optional facts and empty optional sections SHALL be omitted, except
that a verdict count SHALL always be printed with its word even when it is zero.
When a diff moves no debt, the view SHALL be the verdict line only.

Codebase debt detail SHALL fit about one screen at every scope, so zooming in
changes which problems fill the budget and never how much is printed. The budget
SHALL be 24 slots, where a problem card costs one slot plus one slot per shown
evidence line. The budget SHALL be spent through the ladder of card-count and
evidence-line pairs `(6, 3)`, `(8, 2)`, `(12, 1)`, `(24, 0)`, applying the first
rung whose card count is at least the number of cards the displayed scope holds.
Every rung costs exactly the budget, so a scope with few problems shows each in
depth and a scope with many shows more of them with less evidence each. When the
displayed scope holds more cards than the largest rung, the last rung SHALL apply
and the cards ranked below the twenty-fourth SHALL NOT be shown; that cut is the
budget, and `--top` is how a user lifts it. The budget constants and the ladder
are proposed values under review and SHALL be implemented as named constants.

The budget SHALL count slots rather than rendered lines, so the facts a view
states are identical at every terminal width. A row that does not fit the
resolved width SHALL stack its facts on indented lines as it does today, so a
narrow view MAY render more lines than it spends slots.

A cycle witness SHALL cost one slot however many steps it stacks. A witness
states one fact — the path that closes the cycle — and eliding it destroys
that fact rather than shortening it, so the accounting counts the evidence
item and never its steps. The budget is therefore a target a witness MAY
overrun and never a hard line count, and the cards a scope shows and the
evidence each of them states SHALL respect the rung regardless of how many
lines a witness renders.

This replaces the earlier rule that the default view shows the first three ranked
findings or comparisons within the selected scope. That limit capped one section
while the architecture and relationship rows beside it were uncapped, so the
view it produced was neither short nor ranked as a whole. Diff output SHALL keep
the three-comparison limit and its section limits unchanged this round.

`--top N` SHALL provide a middle level of detail between the default and
`--all`. For codebase output it SHALL show at most `N` problem cards and SHALL
select the ladder rung `N` selects by the same rule the default applies to the
scope's card count, including the last-rung fallback when `N` exceeds the
largest rung, so a larger `N` buys breadth by spending evidence depth exactly as
the default does. For diff output it SHALL
raise or lower only the number of displayed ranked comparisons to `N`, leaving
the architecture and history section limits unchanged. `N` SHALL be a positive
integer, and `--top` SHALL conflict with `--json` and with `--all`.

`--all` SHALL show every problem card, `detail` cards included, with complete
evidence and without count limits, and SHALL NOT show raw dependency edges,
standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, or
healthy rows; JSON remains the complete view of those facts.

A selected file scope SHALL show every card anchored on that file with complete
evidence without applying the ladder, because a file holds few cards and drilling
to a file is itself a request for detail.

#### Scenario: Selected scope has more than five affected children
- **WHEN** the default terminal report omits child rows
- **THEN** it shows the five most relevant rows without omitted-row bookkeeping

#### Scenario: A scope holds a handful of problems
- **WHEN** the displayed codebase scope holds five cards
- **THEN** the `(6, 3)` rung applies and each card shows at most three evidence lines

#### Scenario: A scope holds many problems
- **WHEN** the displayed codebase scope holds twenty cards
- **THEN** the `(24, 0)` rung applies and twenty card heads are shown with no evidence lines

#### Scenario: A scope holds more problems than the budget
- **WHEN** the displayed codebase scope holds forty cards and neither `--top` nor `--all` is supplied
- **THEN** the twenty-four highest ranked cards are shown and the rest are cut without bookkeeping rows

#### Scenario: The same scope is rendered at two widths
- **WHEN** one codebase view is rendered at 50 and at 120 columns
- **THEN** both state the same cards and the same evidence, and only row stacking differs

#### Scenario: User requests complete useful terminal detail
- **WHEN** the user supplies `--all`
- **THEN** terminal output shows every card including `detail` ones with complete evidence, and no raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, or healthy rows

#### Scenario: Optional section has no finding
- **WHEN** the displayed scope holds no card the current detail level shows
- **THEN** the terminal omits the problem section

#### Scenario: A diff moves no debt
- **WHEN** no comparison or finding counts as debt movement
- **THEN** the view is the verdict line only, with no trailing history, warning, or context section

#### Scenario: A middle limit is requested
- **WHEN** the user supplies `--top 10` and twelve cards exist in the displayed codebase scope
- **THEN** ten cards are shown with the evidence allowance of the rung that ten selects

#### Scenario: The limit exceeds the cards
- **WHEN** the user supplies `--top 10` and four cards exist
- **THEN** all four are shown and no filler or bookkeeping row appears

#### Scenario: A file is selected
- **WHEN** the selected scope is a file
- **THEN** every card anchored on that file is shown with complete evidence

#### Scenario: A budgeted view holds a long cycle
- **WHEN** a rung allows three evidence lines and one shown `tangle` card carries a witness of twelve steps
- **THEN** the witness renders every step, the card spends one slot on it, and the card count and the other cards' evidence still respect that rung

### Requirement: Codebase detail follows existing hotspot priority
Every displayed problem card whose head or evidence names a finding SHALL show
that finding's unit kind and SHALL show SourceRole whenever the role is not
primary. Recovered advisory findings SHALL use the same ranking inside `--all`
but SHALL remain absent from default detail. Clustering SHALL make that sentence
hold rather than contradict it: a card claims a file's advisory and non-primary
findings like any other, and the card carrying only such findings is the `detail`
card `problem-clustering` defines, so the content it names is ranked and reachable
under `--all` instead of being dropped when no card claims it. Every `detail`
card SHALL be absent from default detail for that one reason, and this
specification SHALL NOT restate the rule that decides visibility.

#### Scenario: A benchmark function is High
- **WHEN** its card appears in default detail
- **THEN** its unit kind and benchmark role are visible

#### Scenario: A recovered method is Watch
- **WHEN** detailed output is requested
- **THEN** its card is present, its unit kind, role when non-primary, and advisory trust are visible, and it holds the position the accepted finding rank gives it

#### Scenario: A healthy file is imported everywhere
- **WHEN** default detail is rendered
- **THEN** its `detail` card is absent and `--all` shows it

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
sentence, using the analysis-owned qualifier bytes. When the verdict carries a
repository-share fact, the share row SHALL render inside the verdict block
directly under the qualifier row when one exists and directly under the tier
sentence otherwise, using the analysis-owned share bytes. It SHALL omit healthy
counts, summary ratios, and decorative quality bars used as data. Coverage gaps
SHALL use one grouped warning sentence only when a gap exists.

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

#### Scenario: A sub-scope view frames the repository
- **WHEN** a package or directory scope is selected and the repository holds High debt
- **THEN** the share row appears inside the verdict block stating the analysis-owned share bytes

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
the report and JSON. Codebase terminal output SHALL present actionable history
findings as problem cards — one card per unexplained coupling finding and one
per knowledge-concentration finding — ranked against every other problem rather
than in a section of their own, and SHALL omit weak pairs and processing totals.
Each card SHALL keep its finding's exact evidence: shared commits, union
commits, similarity, and the dependency state for a coupling pair, and counts
without identity for concentration.

A coupling pair a code dependency already explains is context rather than
debt: it produces no finding, so no card claims it, so codebase terminal
output SHALL NOT state it at any scope or detail level, `--all` included. The
complete pair table with its shared commits, union commits, similarity, and
explanation SHALL remain in the machine report, which is where a reader who
wants context reads it. This narrows the earlier promise that a selected
package or `--all` shows contextual history facts: that promise assumed a
section a renderer filled, and codebase debt detail is now the ranked card
table, which only findings enter.

This replaces the earlier rule that default codebase output shows a `HISTORY`
section of at most three findings ordered by shared commits descending,
similarity descending, then stable package names and IDs. That section-local
order competed with nothing else on the screen, so a Watch coupling pair was
printed beside a High file with no statement of which mattered more. Diff
terminal output SHALL keep `HISTORY` with its accepted limit and order this
round, and with the contextual pairs it shows today.

#### Scenario: Default codebase output has actionable history
- **WHEN** an unexplained coupling finding exists
- **THEN** one card states that pair once with its exact commit evidence and without a retained-pair or processing summary, ranked among the other problems

#### Scenario: A user selects an evolution detail target in a diff
- **WHEN** a package is selected or `--all` is supplied in diff output
- **THEN** relevant actionable and contextual history facts are shown without healthy rows, weak default pairs, or internal processing facts

#### Scenario: A code dependency explains a coupling pair
- **WHEN** a codebase package scope is selected or `--all` is supplied and a retained pair has a code dependency
- **THEN** no terminal row states that pair and the machine report keeps its complete row

### Requirement: Architecture default shows witnesses rather than edge samples
Codebase output SHALL present a rated cycle as one problem card carrying the
existing rated cycle witness as evidence, and SHALL show no edge rows and no
edge totals. Each shown witness SHALL state its severity in the word vocabulary,
`high` or `watch`, optionally decorated with its glyph when decoration is
enabled. This supersedes the earlier rule that witnesses appear without severity
words and with a status glyph. Diff output SHALL keep its `ARCHITECTURE` section
with the same witness rule.

This replaces the earlier sentence that `--all` and path drill expose relevant
relationship detail. Neither `--all` nor a path view SHALL expose relationship
rows at any scope; a relationship reaches a human only as aggregate card
evidence, and the machine report retains the complete relation tables.

#### Scenario: An acyclic graph has many edges
- **WHEN** default output is rendered
- **THEN** no cycle card and no edge total appears, and no detail level lists the edges

#### Scenario: A rated cycle exists
- **WHEN** default output is rendered
- **THEN** one card states the cycle once with its severity word and its witness, decorated only when decoration is enabled

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

