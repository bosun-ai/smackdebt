# terminal-output Specification

## Purpose
TBD - created by archiving change simplify-terminal-output. Update Purpose after archive.
## Requirements
### Requirement: Human status uses one exact glyph vocabulary
Human terminal output SHALL state severity, direction, and diagnostic meaning in words: `high`,
`watch`, `worse`, `better`, `changed`, `warning`, and `next:`. Every meaning
SHALL be readable from words alone.

Glyphs SHALL become decoration. When decorations are enabled, output MAY use
High U+F024, Watch U+F0EB, Discover U+F46B, Worse U+F062, Better U+F063,
Changed U+F111, Warning U+F071, and the tier bar `▌`. Each glyph SHALL occupy
one display cell and SHALL appear immediately adjacent to the word it decorates,
never in place of that word. U+EC3F SHALL NOT appear.

This requirement REVERSES the previously accepted rule that human output SHALL
NOT repeat a severity or direction word beside a glyph. That rule assumed a
single terminal audience; the report is now read by humans and by machine
consumers through a pipe, so the word is the meaning and the glyph is optional
decoration of it. No fact SHALL be spelled twice in two vocabularies: one word,
optionally decorated, states each fact once.

When decorations are disabled, output SHALL contain no glyph and no tier bar,
and SHALL contain no codepoint in the range U+E000–U+F8FF.

#### Scenario: A report contains every status
- **WHEN** decorated terminal output is inspected by Unicode scalar value and display width
- **THEN** each meaning appears as its word, any glyph is exact, one cell, and immediately adjacent to that word, and U+EC3F is absent

#### Scenario: Output is piped
- **WHEN** standard output is not a terminal
- **THEN** every severity, direction, and diagnostic is stated in words and no codepoint in U+E000–U+F8FF appears

#### Scenario: Decorated and undecorated output are compared
- **WHEN** the same report is rendered with and without decorations
- **THEN** both state the same facts in the same words and only glyphs, the tier bar, and ANSI sequences differ

### Requirement: Color applies only to status glyphs
Terminal decoration SHALL be resolved beside color from the existing terminal detection
without adding a CLI option. With decorations and color enabled, High and Worse
glyphs SHALL be red, Watch and Warning glyphs SHALL use ANSI-256 color 208,
Discover SHALL be cyan, Better SHALL be green, Changed SHALL use normal text
color, and the verdict bar SHALL use its tier color. Styling SHALL reset
immediately after each decorated element so adjacent text stays normal.
`--color never` SHALL produce fully plain output with no ANSI sequence.
Color-disabled, `NO_COLOR`, and redirected output SHALL retain the same visible
words without ANSI.

#### Scenario: Forced color is compared with plain output
- **WHEN** the same command runs with `--color always` and `--color never`
- **THEN** ANSI sequences surround only decorated elements and stripping ANSI produces bytes whose words are identical

#### Scenario: Automatic color is disabled
- **WHEN** output is redirected or `NO_COLOR` is present
- **THEN** no ANSI sequence appears and every meaning remains readable in words

### Requirement: Default sections show only relevant decisions
Default terminal output SHALL open with a verdict block written from completed verdict facts.
The block SHALL show the selected scope, the analysis-owned tier sentence, the
counts behind it with every count labeled by its word, and the worst offender
with its resolved repository-relative path and its reason when one exists. When
the completed verdict carries a repository-share fact, the block SHALL print its
analysis-owned bytes verbatim. The renderer SHALL NOT compose a sentence, derive
a tier, or compute a count.

For diff output the verdict block SHALL label every count with its word, SHALL
name the comparison family that moved, and SHALL print a zero count rather than
omitting it. When a diff moves no debt, output SHALL be the verdict line only,
with no area, finding, architecture, history, problem, or warning section
following it.

`AREAS` SHALL appear only when several debt-bearing child areas exist, with at
most five rows and word-labeled counts. The discover line SHALL be
`next: smackdebt <path>`.

Codebase output SHALL render one `PROBLEMS` section in place of `FINDINGS`,
`ARCHITECTURE`, and `HISTORY`. This REVERSES the previously accepted rule that
codebase output shows those three sections. That rule gave every finding, every
architecture finding, and every history finding a row of its own, so one file
with three High findings was named three times, architecture findings were cut
to three without ranking them against each other, and no line ever told the
reader what the problem was. One ranked section of named problems states each
problem once. `PROBLEMS` SHALL appear only when the displayed scope holds at
least one card that the current detail level shows.

Each problem row SHALL state its rating word, then its pattern name, then its
anchor. Pattern names SHALL be exactly `does too much` for `god_file`,
`everything depends on this` for `hub`, `circular dependency` for `tangle`,
`hot and complex` for `hot_mess`, `changes together` for `shotgun_pair`,
`one author` for `bus_risk`, and `depends on less stable code` for
`unstable_dependency`. A `measured` card's head SHALL be the identity of its top
claimed finding, which is the head a finding row states today. A card that
claims no finding SHALL state no rating word: the word vocabulary rates debt,
a card claiming nothing carries none, and `watch` would misstate it.

An anchor SHALL be written by its kind, reusing the identity the accepted specs
already give that kind:

- **A single file** — `god_file`, `hub`, `hot_mess`, and `measured` — SHALL be
  its repository-relative file path, written `path:line` when the card's head is
  a claimed finding that carries a span.
- **A file set** — `tangle` — SHALL be the cycle's first witness path, the same
  identity the accepted worst-offender rule names for a cycle, with the complete
  witness following as stacked evidence.
- **A package pair** SHALL keep its family's accepted wording, because one
  relationship is symmetric and the other is not: `shotgun_pair` SHALL read
  `<left> ↔ <right>` as the accepted coupling wording does, and
  `unstable_dependency` SHALL read `<source> → <target>` as the accepted
  stable-dependency row does.
- **A single package** — `bus_risk` — SHALL be its repository-relative package
  name, written `repository root` for package path `.` as every other heading,
  row, and breadcrumb writes it.

Card evidence SHALL render one indented line per shown evidence item, in the
order analysis stored it, with one exact wording per evidence kind:
`<n> files import this`, `imports <n> files`, `hot (<n> commits)`,
`<n> rated units`, `<n> files in the cycle`, a claimed finding's `path:line`
with its measurements, a size finding's subject and measured value, a
stable-dependency finding's integer degree operands and reference count, a
coupling pair's shared commits, union commits, similarity operands, and
dependency state, and a knowledge-concentration finding's counts without
identity. Clustering carries a touch count only for a hotspot, so heat is the
only activity wording a card states; the activity of a file that is not a
hotspot remains a machine-report fact. Every wording SHALL use correct singular
and plural form, in the verb and in the object alike. A `tangle` card SHALL
carry its architecture finding's existing cycle witness as stacked evidence,
and that witness SHALL NOT be shortened with an ellipsis.

When a card's head already names the finding an evidence item links — which
only a `measured` card's first item does, because that head is that finding —
the line SHALL state what the head has not: the finding's measurements, or a
size finding's measured value alone. The head owns the identity, the unit
kind, the source role, the parse trust, and the location, and each fact
reaches a reader once. A `measured` card headed on a size finding SHALL
therefore write that finding's identity in the same `<identity> · <kind>` form
a finding-headed card writes, which for a file's own length is the file path
followed by `file`, and for a container is the container name followed by
`container` and then the anchor.

Diff output SHALL keep `FINDINGS`, `ARCHITECTURE`, and `HISTORY` this round.
`FINDINGS` SHALL show ranked comparison rows with `path:line` and their
measurements. `ARCHITECTURE` SHALL appear only when an architecture comparison
exists and SHALL show cycle witnesses rather than edge totals. `HISTORY` SHALL
appear only when an actionable history finding exists, SHALL show at most three,
SHALL show one row per package pair, and SHALL show knowledge-concentration rows
as counts without identity.

#### Scenario: A codebase report is rendered
- **WHEN** default codebase output is written
- **THEN** it opens with the scope, the tier sentence, word-labeled counts, and the worst offender with its path and reason, and its debt detail is one `PROBLEMS` section

#### Scenario: One file carries three High findings
- **WHEN** default codebase output is written for a scope containing that file
- **THEN** one problem row names that file once instead of three finding rows naming it three times

#### Scenario: A cycle is reported
- **WHEN** an architecture finding covers a strongly connected component
- **THEN** one `circular dependency` row states the member count and carries the existing witness as stacked evidence with no ellipsis

#### Scenario: A problem is in a hot file
- **WHEN** a displayed card's file is a hotspot with 14 commits
- **THEN** one evidence line states `hot (14 commits)`

#### Scenario: A card heads on the finding its evidence links
- **WHEN** a `measured` card's head names its top claimed finding and that finding is its first evidence item
- **THEN** the evidence line states that finding's measurements alone, and its path, line, unit kind, role, and trust appear once, in the head

#### Scenario: A file is measured only by its length
- **WHEN** a file carries a size finding and no unclaimed source finding
- **THEN** its `measured` card heads on `<path> · file` and its evidence line states the measured value alone

#### Scenario: A card claims nothing
- **WHEN** a `hub` card that claims no finding is shown
- **THEN** its row states its pattern name and its anchor with no rating word

#### Scenario: A diff moves debt in one family
- **WHEN** a diff introduces a package cycle and moves no source comparison
- **THEN** the verdict block states the worse count with its word, names the architecture family, prints the better and changed counts as zero rather than omitting them, and the diff view keeps its `FINDINGS`, `ARCHITECTURE`, and `HISTORY` sections

#### Scenario: A diff moves no debt
- **WHEN** no comparison or finding counts as debt movement
- **THEN** output is the verdict line only and no further section is written

### Requirement: Human output removes repeated and internal facts
Human terminal output SHALL omit bars used as data, area percentages and ratios, healthy
counts, raw dependency edges, standard-library externals, churn dumps,
cyclomatic-1 rows, weak coupling rows, arbitrary architecture edge totals, and
history processing totals. It SHALL NOT use the phrases `complete local
stream`, `eligible mapping`, `retained units`, or `retained package pairs`.
Cognitive, cyclomatic, and statement values SHALL remain on applicable source
findings, and the human terminal SHALL call the logical-line measurement
`statements`. Each fact SHALL appear once, stated in words.

The omission of raw dependency edges SHALL be absolute: no human view SHALL
print a dependency edge as a row at any scope, in any mode, at any detail level,
including `--all` and a selected path. A dependency relationship SHALL reach a
human view only as aggregate problem evidence, such as a fan-in or fan-out count
or a cycle witness. Complete relation tables SHALL remain in the machine report.

#### Scenario: A report contains source, graph, and history detail
- **WHEN** default output is rendered
- **THEN** each actionable fact appears once in words and no bar-as-data, ratio, healthy row, raw edge, or processing fact appears

#### Scenario: Every human view is scanned for edge rows
- **WHEN** every committed human view is scanned at every scope and detail level
- **THEN** no row heads two repository file paths with an arrow or with ` owns `, no line states the single-reference import fact `· 1 import`, and the only arrows joining two repository file paths are cycle-witness evidence

### Requirement: Detailed and path views remain useful
`--all` SHALL show all useful debt, including `detail` problem cards, and SHALL NOT show raw dependency
edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak
coupling, or healthy rows; those SHALL remain available only in JSON.

This REVERSES the previously accepted rule that a selected path retains relevant
incoming and outgoing debt-bearing relationships even when one endpoint lies
outside that path. That rule assumed a relationship deserved a row of its own.
Over a 296-file directory it produced about 1,270 unrated, unranked rows and
named no problem, and at every scope below the repository it was the reason
zooming in made the report longer. Relationship detail is now aggregate problem
evidence in the terminal and a complete table in JSON.

Human activity rows SHALL say `commit` or `commits`. The resolved-use wording
`<source> → <target> · 1 import` and the ownership wording `<source> owns
<target>` SHALL be removed together with the rows that carried them. Unresolved
and ambiguous rows SHALL say `could not be matched` and `matched more than one
file`, and SHALL appear only when `--all` is supplied or the selected scope is a
file; at every other scope the grouped `WARNINGS` sentence SHALL be their whole
terminal presence. Primary and trusted SHALL be omitted; non-primary roles and
advisory trust SHALL appear only when useful.

Every coupling row SHALL retain shared commits, union commits, similarity, and
end with `code dependency exists` or `no code dependency`. A worse coupling
comparison SHALL say `<left> ↔ <right> now change together without a code
dependency`; a better comparison SHALL say `<left> ↔ <right> no longer change
together without a code dependency`. Human output SHALL NOT expose the internal
coupling comparison label.

Every diff finding SHALL show `path:line`, SHALL show before and after values for
each changed measurement, and SHALL use a human identity derived from file and
unit kind for an anonymous unit. Generated internal identities such as
`<closure 1177>` SHALL NOT appear in human output.

#### Scenario: User requests all detail
- **WHEN** `--all` is supplied
- **THEN** count limits on useful debt are removed, `detail` cards become visible, and raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, and healthy rows remain absent

#### Scenario: User drills into a path
- **WHEN** a dependency relationship crosses the selected path boundary
- **THEN** no relationship row is printed and the relationship reaches the reader only through a problem card's aggregate evidence

#### Scenario: An import could not be followed at a directory scope
- **WHEN** a directory scope is selected without `--all` and it contains unresolved imports
- **THEN** the grouped `WARNINGS` sentence states them and no per-import row appears

#### Scenario: An import could not be followed at a file scope
- **WHEN** the selected scope is a file, or `--all` is supplied at any scope
- **THEN** the per-import rows say `could not be matched` and `matched more than one file`

#### Scenario: A diff changes an anonymous unit
- **WHEN** a comparison covers a closure or template unit without a source name
- **THEN** its row shows a human identity such as `GraphEditor.vue · closure`, its `path:line`, and each changed measurement as before and after

### Requirement: Human diagnostics are grouped and simple
Warnings SHALL be grouped into one sentence per kind and SHALL be introduced by
the word `warning`. Incomplete history SHALL be written as
`History is incomplete.` Rename gaps SHALL be written as
`Some renamed files could not be matched.` Unmatched and ambiguous imports
SHALL be combined into one sentence that states the total and then each cause
only when its count is non-zero, such as
`29 imports could not be followed · 24 named nothing in the repository · 5 matched more than one file`,
with correct singular and plural wording throughout.

Anonymous-unit match ambiguity SHALL be grouped separately from import
ambiguity. It SHALL count affected files once and state either
`1 file has anonymous units that could not be matched safely.` or
`<n> files have anonymous units that could not be matched safely.` One rendered
scope SHALL contain at most one such aggregate warning,
however many collision groups a file holds. It SHALL NOT expose an anchor, digest, parser node, candidate count,
or guessed direction. File detail SHALL remain available in `--all` and at a
selected file scope through the existing ambiguity comparison and diagnostic.

An empty history window SHALL be disclosed as
`No commits in the last <n> days.` parameterized on the selected window length,
emitted when the history stream is complete, a window is selected, and zero
commits fall inside it; the sentence SHALL follow availability warnings and
precede rename gaps. Nested repositories SHALL be disclosed in default output
with their own subject, `1 nested repository was not analyzed.` with correct
plural form. No warning SHALL expose command, status, parser, process, or other
implementation text, and no warning SHALL repeat its summary sentence for each
fact.

#### Scenario: Several files share a warning kind
- **WHEN** default output is rendered
- **THEN** one grouped sentence states the count for that kind instead of one line per file

#### Scenario: Imports could not be followed for both reasons
- **WHEN** 24 unresolved and 5 ambiguous imports are retained
- **THEN** one warning row states `29 imports could not be followed · 24 named nothing in the repository · 5 matched more than one file` with no implementation text

#### Scenario: Anonymous units are ambiguous in two files
- **WHEN** two files retain unsafe anonymous-unit match groups
- **THEN** one warning row states `2 files have anonymous units that could not be matched safely.` and neither file is counted twice

#### Scenario: One file has several collision groups
- **WHEN** one file retains three unsafe anonymous-unit match groups
- **THEN** one warning row states `1 file has anonymous units that could not be matched safely.`

#### Scenario: The history window is empty
- **WHEN** a complete stream yields zero commits inside a selected 90-day window
- **THEN** one warning row states `No commits in the last 90 days.` and the problem section still renders

#### Scenario: Nested repositories were pruned
- **WHEN** two nested checkouts were excluded from discovery
- **THEN** default output states `2 nested repositories were not analyzed.` without naming files as the subject

### Requirement: Human command text uses the same simple language
The system SHALL use short user-facing language for CLI help, argument errors,
runtime errors, terminal labels, and README examples and SHALL avoid internal
report, composition, stream, mapping, command, parser, and process terms.

A missing selected path SHALL exit with status 1, write empty stdout, and write
exact stderr bytes `smackdebt: path not found: <user-path>\n`. An existing
selected directory containing no recognized source SHALL do the same with
`smackdebt: no source files found under: <user-path>\n`. An existing selected
non-source file SHALL do the same with
`smackdebt: not a source file: <user-path>\n`. Each path error SHALL retain the
original user input without an absolute path or operating-system error text.

A missing Git ref SHALL exit with status 1, write empty stdout, and write exact
stderr bytes `smackdebt: Git ref not found: <ref>\n` without Git command,
process status, or fatal output. `--all --json` SHALL exit with status 2, write
empty stdout, and write exact stderr bytes
`smackdebt: --all cannot be used with --json\n`. None of these failures SHALL
append usage or help.

`--top` SHALL reject zero and SHALL conflict with `--json` and with `--all`,
each rejection exiting with status 2 in the same short language. A missing gate
baseline SHALL exit with status 2, write empty stdout, and write exact stderr
bytes `smackdebt: baseline not found: <path>\n` with no usage or help tail. Exit
status 3 SHALL mean the gate baseline was exceeded, and the gate's proposal line
SHALL read `next: smackdebt gate --update` in the same command vocabulary as the
codebase discover line.

#### Scenario: Missing selected path is reported
- **WHEN** the user selects a path that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: path not found: <user-path>\n` with no usage, help, or operating-system text

#### Scenario: Source-free directory is reported
- **WHEN** the user selects an existing directory containing no recognized source
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: no source files found under: <user-path>\n`

#### Scenario: Non-source file is reported
- **WHEN** the user selects an existing file that is not recognized source
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: not a source file: <user-path>\n`

#### Scenario: Missing Git ref is reported
- **WHEN** the user selects a Git ref that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: Git ref not found: <ref>\n` with no usage or help tail

#### Scenario: All detail conflicts with JSON
- **WHEN** the user supplies `--all --json`
- **THEN** status is 2, stdout is empty, and stderr is exactly `smackdebt: --all cannot be used with --json\n` with no usage or help tail

#### Scenario: A gate baseline is missing
- **WHEN** the user runs the gate without a baseline and without requesting an update
- **THEN** status is 2, stdout is empty, and stderr is exactly `smackdebt: baseline not found: <path>\n` with no usage or help tail

#### Scenario: An invalid top limit is rejected
- **WHEN** the user supplies `--top 0`, `--top` with `--json`, or `--top` with `--all`
- **THEN** each invocation exits with status 2 and a short product-language error

### Requirement: Machine and analysis interfaces do not change
The system SHALL preserve the current JSON contract's bytes, report facts, finding rank, analysis
policy, exit classes, serial and automatic behavior, live work counts,
allocations, and measured resource behavior unchanged through this terminal
redesign. The renderer SHALL perform no analysis and SHALL consume completed
verdict, presentation, and report facts only.

#### Scenario: Terminal bytes change
- **WHEN** the same generated fixtures are rendered as JSON and terminal output
- **THEN** only human terminal and error bytes differ and every machine, analysis, work, and resource check still passes

### Requirement: A short mixed diff represents its conclusion
When the default diff verdict is `mixed`, presentation SHALL select comparison
rows linked by the selected scope's debt-diff selection across source,
architecture, and history before applying one view-wide limit. The exact key
SHALL be direction Worse, Better, Changed; family source, architecture,
history; repository-relative subject path or path pair; start line with absent
after present; family kind in enum order; then comparison identity.

The default limit SHALL remain three. Default mixed selection SHALL reserve the
first row by that key for each present direction, then fill remaining positions
from the same key without duplicates. A reserved row SHALL remain in its owning
section, whose chosen rows use the same direction and stable-subject order.
Current-state history context that is not a diff comparison SHALL keep its
existing section-local handling and SHALL NOT consume the comparison limit.

`--all` SHALL retain all useful rows. An explicit `--top <n>` SHALL never emit
more than `n` debt-comparison rows across all three families and SHALL select the
first `n` rows by the exact key without reservation. No view SHALL add an
omitted-row notice.

Every diff that renders a comparison section or warning SHALL end, after any
warnings, with the exact undecorated line
`  inspect directories and files for more details\n`. It SHALL NOT print a
`next:` line. A `no_debt_change` diff SHALL remain verdict-only unless one of
these comparison-trust facts exists: an incomplete-coverage qualifier or
warning, an anonymous-match warning, a suppressed graph-comparison warning, or
a rename-gap or history-availability warning that affects the selected
comparison. When one exists, the report SHALL render only the applicable
qualifier and comparison-trust warning rows after the verdict and SHALL end with
the same detail footer. Unrelated current-state areas, history findings,
concentration, coupling, and other context SHALL NOT pierce verdict-only output.

#### Scenario: Worse and better source rows compete for the default limit
- **WHEN** a mixed diff has enough worse rows to fill the report and at least one better row
- **THEN** the default view includes the highest-ranked worse row and the highest-ranked better row

#### Scenario: Directions span comparison families
- **WHEN** source has Worse, architecture has Better, and history has Changed rows
- **THEN** the default view includes one row from each present direction in its owning section

#### Scenario: One row is requested
- **WHEN** the user supplies `--top 1` to a mixed diff
- **THEN** exactly one ranked row is visible and the report does not expand to represent every direction

#### Scenario: Diff detail is rendered
- **WHEN** a terminal diff has at least one visible comparison section or warning
- **THEN** its final bytes are exactly `  inspect directories and files for more details\n` and it contains no `next:` line

#### Scenario: No debt changed
- **WHEN** a fully checked diff has the `no_debt_change` verdict and no comparison-trust warning
- **THEN** the output ends after `No debt changed.` without the detail footer

#### Scenario: No debt changed but source was not all checked
- **WHEN** a diff has the `no_debt_change` verdict and an incomplete-coverage qualifier or warning
- **THEN** the qualifier and warning remain visible and the report ends with `  inspect directories and files for more details\n`

#### Scenario: No debt changed but graph evidence was withheld
- **WHEN** a diff has the `no_debt_change` verdict and a suppressed graph comparison warning
- **THEN** the warning remains visible and the report ends with the detail footer

#### Scenario: A documentation-only diff retains current history context
- **WHEN** a documentation-only change moves no debt while the current repository retains coupling, concentration, or other history findings unrelated to the comparison
- **THEN** terminal output is exactly the verdict form ending in `No debt changed.` with no section or footer

#### Scenario: Comparison history is incomplete
- **WHEN** a no-debt comparison has a rename gap or history-availability warning that affects its trust
- **THEN** that warning remains visible, unrelated current-state context stays hidden, and the report ends with the detail footer

