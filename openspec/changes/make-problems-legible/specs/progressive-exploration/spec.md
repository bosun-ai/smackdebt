## MODIFIED Requirements

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

`--all` SHALL show every problem card, descriptive cards included, with complete
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
- **THEN** terminal output shows every card including descriptive ones with complete evidence, and no raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, or healthy rows

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

### Requirement: Codebase detail follows existing hotspot priority
Every displayed problem card whose head or evidence names a finding SHALL show
that finding's unit kind and SHALL show SourceRole whenever the role is not
primary. Recovered advisory findings SHALL use the same ranking inside `--all`
but SHALL remain absent from default detail, and descriptive cards SHALL be
absent from default detail for the same reason.

#### Scenario: A benchmark function is High
- **WHEN** its card appears in default detail
- **THEN** its unit kind and benchmark role are visible

#### Scenario: A recovered method is Watch
- **WHEN** detailed output is requested
- **THEN** its unit kind, role when non-primary, and advisory trust are visible

#### Scenario: A healthy file is imported everywhere
- **WHEN** default detail is rendered
- **THEN** its descriptive card is absent and `--all` shows it

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

### Requirement: Reports expose evolution as a separate concern
The system SHALL retain history coverage, churn, coupling, and concentration in
the report and JSON. Codebase terminal output SHALL present actionable history
findings as problem cards — one card per unexplained coupling finding and one
per knowledge-concentration finding — ranked against every other problem rather
than in a section of their own, and SHALL omit weak pairs and processing totals.
Each card SHALL keep its finding's exact evidence: shared commits, union
commits, similarity, and the dependency state for a coupling pair, and counts
without identity for concentration.

This replaces the earlier rule that default codebase output shows a `HISTORY`
section of at most three findings ordered by shared commits descending,
similarity descending, then stable package names and IDs. That section-local
order competed with nothing else on the screen, so a Watch coupling pair was
printed beside a High file with no statement of which mattered more. Diff
terminal output SHALL keep `HISTORY` with its accepted limit and order this
round.

#### Scenario: Default codebase output has actionable history
- **WHEN** an unexplained coupling finding exists
- **THEN** one card states that pair once with its exact commit evidence and without a retained-pair or processing summary, ranked among the other problems

#### Scenario: A user selects an evolution detail target
- **WHEN** a package is selected or `--all` is supplied
- **THEN** relevant actionable and contextual history facts are shown without healthy rows, weak default pairs, or internal processing facts

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
