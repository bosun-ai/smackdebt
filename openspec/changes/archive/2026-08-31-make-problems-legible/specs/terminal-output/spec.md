## MODIFIED Requirements

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
Warnings SHALL be grouped into one sentence per kind and SHALL be introduced by the word
`warning`. Incomplete history SHALL be written as `History is incomplete.`
Rename gaps SHALL be written as `Some renamed files could not be matched.`
Unmatched and ambiguous imports SHALL be combined into one sentence that
states the total and then each cause only when its count is non-zero, such as
`29 imports could not be followed · 24 named nothing in the repository · 5
matched more than one file`, with correct singular and plural wording
throughout. An empty history window SHALL be disclosed as `No commits in the
last <n> days.` parameterized on the selected window length, emitted when the
history stream is complete, a window is selected, and zero commits fall inside
it; the sentence SHALL follow the availability warnings and precede rename
gaps. Nested repositories SHALL be disclosed in default output with their own
subject, `1 nested repository was not analyzed.` with correct plural form, and
each diagnostic kind SHALL name its own subject rather than reusing another
kind's. No warning SHALL expose command, status, parser, process, or other
implementation text, and no warning SHALL repeat its summary sentence for each
fact. File detail SHALL remain available in `--all` or at a file scope; this
replaces the earlier promise of file detail in any path view, because a path
view above a file is exactly the view the grouped sentence exists for.

#### Scenario: Several files share a warning kind
- **WHEN** default output is rendered
- **THEN** one grouped sentence states the count for that kind instead of one line per file

#### Scenario: Imports could not be followed for both reasons
- **WHEN** 24 unresolved and 5 ambiguous imports are retained
- **THEN** one warning row states `29 imports could not be followed · 24 named nothing in the repository · 5 matched more than one file` with no implementation text

#### Scenario: Imports could not be followed for one reason
- **WHEN** only unresolved imports are retained
- **THEN** the sentence states the total and the unresolved fact and omits the zero-count cause

#### Scenario: The history window is empty
- **WHEN** a complete stream yields zero commits inside a selected 90-day window
- **THEN** one warning row states `No commits in the last 90 days.` and the problem section still renders

#### Scenario: Nested repositories were pruned
- **WHEN** two nested checkouts were excluded from discovery
- **THEN** default output states `2 nested repositories were not analyzed.` without naming files as the subject

#### Scenario: A package view holds unfollowed imports
- **WHEN** a package scope is rendered without `--all`
- **THEN** the grouped sentence is the whole terminal presence of those imports and no file detail row appears
