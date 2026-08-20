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
with its resolved repository-relative path and its reason when one exists. The
renderer SHALL NOT compose a sentence, derive a tier, or compute a count.

For diff output the verdict block SHALL label every count with its word, SHALL
name the comparison family that moved, and SHALL print a zero count rather than
omitting it. When a diff moves no debt, output SHALL be the verdict line only,
with no area, finding, architecture, history, or warning section following it.

`AREAS` SHALL appear only when several debt-bearing child areas exist, with at
most five rows and word-labeled counts. `FINDINGS` SHALL show ranked rows with
`path:line` and their measurements, and SHALL append `· hot (<n> commits)` when
the finding's file is a hotspot. `ARCHITECTURE` SHALL appear only when an
architecture finding exists, SHALL show cycle witnesses rather than edge totals,
and SHALL show stable-dependency rows with their integer operands. `HISTORY`
SHALL appear only when an actionable history finding exists, SHALL show at most
three, SHALL show one row per package pair, and SHALL show
knowledge-concentration rows as counts without identity. The discover line SHALL
be `next: smackdebt <path>`.

#### Scenario: A codebase report is rendered
- **WHEN** default codebase output is written
- **THEN** it opens with the scope, the tier sentence, word-labeled counts, and the worst offender with its path and reason

#### Scenario: A diff moves debt in one family
- **WHEN** a diff introduces a package cycle and moves no source comparison
- **THEN** the verdict block states the worse count with its word, names the architecture family, and prints the better and changed counts as zero rather than omitting them

#### Scenario: A diff moves no debt
- **WHEN** no comparison or finding counts as debt movement
- **THEN** output is the verdict line only and no further section is written

#### Scenario: A finding is in a hot file
- **WHEN** a ranked finding's file is a hotspot with 14 commits
- **THEN** its row states `hot (14 commits)` beside its identity, location, and measurements

### Requirement: Human output removes repeated and internal facts
Human terminal output SHALL omit bars used as data, area percentages and ratios, healthy
counts, raw dependency edges, standard-library externals, churn dumps,
cyclomatic-1 rows, weak coupling rows, arbitrary architecture edge totals, and
history processing totals. It SHALL NOT use the phrases `complete local
stream`, `eligible mapping`, `retained units`, or `retained package pairs`.
Cognitive, cyclomatic, and statement values SHALL remain on applicable source
findings, and the human terminal SHALL call the logical-line measurement
`statements`. Each fact SHALL appear once, stated in words.

#### Scenario: A report contains source, graph, and history detail
- **WHEN** default output is rendered
- **THEN** each actionable fact appears once in words and no bar-as-data, ratio, healthy row, raw edge, or processing fact appears

### Requirement: Detailed and path views remain useful
`--all` SHALL show all useful debt and SHALL NOT show raw dependency edges, standard-library
externals, churn dumps, cyclomatic-1 rows, weak coupling, or healthy rows; those
SHALL remain available only in JSON. A selected path SHALL retain relevant
incoming and outgoing debt-bearing relationships even when one endpoint lies
outside that path.

Human activity rows SHALL say `commit` or `commits`. Resolved uses SHALL say
`<source> → <target> · 1 import` with correct plural form; ownership SHALL say
`<source> owns <target>`; unresolved and ambiguous rows SHALL say `could not be
matched` and `matched more than one file`. Primary and trusted SHALL be omitted;
non-primary roles and advisory trust SHALL appear only when useful.

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
- **THEN** count limits on useful debt are removed while raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, and healthy rows remain absent

#### Scenario: User drills into a path
- **WHEN** a debt-bearing relationship crosses the selected path boundary
- **THEN** the relevant incoming or outgoing relationship remains visible

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
fact. File detail SHALL remain available in `--all` or a path view.

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
- **THEN** one warning row states `No commits in the last 90 days.` and source and architecture sections still render

#### Scenario: Nested repositories were pruned
- **WHEN** two nested checkouts were excluded from discovery
- **THEN** default output states `2 nested repositories were not analyzed.` without naming files as the subject

### Requirement: Human command text uses the same simple language
The system SHALL use short user-facing language for CLI help, argument errors, runtime
errors, terminal labels, and README examples and SHALL avoid internal report,
composition, stream, mapping, command, parser, and process terms.

A missing selected path SHALL exit with status 1, write empty stdout, and write
exact stderr bytes `smackdebt: path not found: <user-path>\n` from the original
user input without an absolute path or operating-system error text. A missing
Git ref SHALL exit with status 1, write empty stdout, and write exact stderr
bytes `smackdebt: Git ref not found: <ref>\n` without a Git command, process
status, or fatal output. `--all --json` SHALL exit with status 2, write empty
stdout, and write exact stderr bytes `smackdebt: --all cannot be used with
--json\n`. None of these failures SHALL append usage or help.

`--top` SHALL reject zero and SHALL conflict with `--json` and with `--all`,
each rejection exiting with status 2 in the same short language. A missing
gate baseline SHALL exit with status 2, write empty stdout, and write exact
stderr bytes `smackdebt: baseline not found: <path>\n` with no usage or help
tail. Exit status 3 SHALL mean the gate baseline was exceeded, and the gate's
proposal line SHALL read `next: smackdebt gate --update` in the same command
vocabulary as the discover line.

#### Scenario: Missing selected path is reported
- **WHEN** the user selects a path that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: path not found: <user-path>\n` with no usage, help, or operating-system text

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
