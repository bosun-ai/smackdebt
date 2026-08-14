# terminal-output Specification

## Purpose
TBD - created by archiving change simplify-terminal-output. Update Purpose after archive.
## Requirements
### Requirement: Human status uses one exact glyph vocabulary
Terminal output SHALL use High U+F024, Watch U+F0EB, Discover U+F46B, Worse
U+F062, Better U+F063, Changed U+F111, and Warning U+F071. Each glyph SHALL
occupy one display cell. U+EC3F SHALL NOT appear. Human output SHALL NOT repeat
severity or direction words beside a glyph.

#### Scenario: A report contains every status
- **WHEN** plain terminal output is inspected by Unicode scalar value and display width
- **THEN** each meaning uses its exact glyph in one cell, U+EC3F is absent, and no redundant status word follows a glyph

### Requirement: Color applies only to status glyphs
With color enabled, High and Worse glyphs SHALL be red, Watch and Warning
glyphs SHALL use ANSI-256 color 208, Discover SHALL be cyan, Better SHALL be
green, and Changed SHALL use normal text color. Styling SHALL reset immediately
after each glyph so adjacent text stays normal. Color-disabled, `NO_COLOR`, and
redirected output SHALL retain the same visible Unicode text without ANSI.

#### Scenario: Forced color is compared with plain output
- **WHEN** the same command runs with `--color always` and `--color never`
- **THEN** ANSI sequences surround only the specified glyphs and stripping ANSI produces identical bytes

#### Scenario: Automatic color is disabled
- **WHEN** output is redirected or `NO_COLOR` is present
- **THEN** no ANSI sequence appears and every required glyph remains present

### Requirement: Default sections show only relevant decisions
Default terminal output SHALL always show `QUALITY`. It SHALL show `AREAS` only
when several affected child areas exist, with at most five rows. It SHALL show
the first three ranked rows under `FINDINGS`. It SHALL show `ARCHITECTURE` only
when an architecture finding exists and SHALL show cycle witnesses rather than
edge totals. It SHALL show `HISTORY` only when an actionable history finding
exists. The discover line SHALL contain only the Discover glyph and the next
command. Default `HISTORY` SHALL show at most three actionable findings ordered
by shared commits descending, similarity descending, then stable package names
and IDs. Each row SHALL use `<left> ↔ <right> changed together in <shared> of
<union> commits · <similarity>% · no code dependency` with the Watch glyph and
no repeated severity word.

#### Scenario: One affected area and no architecture or history finding
- **WHEN** default codebase output is rendered
- **THEN** `QUALITY` and ranked `FINDINGS` appear while `AREAS`, `ARCHITECTURE`, and `HISTORY` are absent

#### Scenario: Several affected areas and every finding family
- **WHEN** default codebase output is rendered
- **THEN** at most five areas, three ranked source findings, architecture witnesses, at most three actionable history findings with exact evidence, and one glyph-plus-command discover line appear

#### Scenario: Diff output has relevant findings
- **WHEN** Worse, Better, Changed, architecture, or history findings exist
- **THEN** the same relevance and detail limits apply without a retained-unit summary

### Requirement: Human output removes repeated and internal facts
Human terminal output SHALL omit repeated bars, summary percentages and ratios, healthy
counts, severity words, arbitrary architecture edge totals, weak history pairs,
and history processing totals. It SHALL NOT use the phrases `complete local
stream`, `eligible mapping`, `retained units`, or `retained package pairs`.
Cognitive, cyclomatic, and statement values SHALL remain on applicable source
findings. The human terminal SHALL call the logical-line measurement
`statements` so its meaning is clear without reading documentation.
The actionable history evidence ratio is not a summary ratio and SHALL remain.

#### Scenario: A report contains source, graph, and history detail
- **WHEN** default output is rendered
- **THEN** each actionable fact appears once without the removed labels or processing facts and source findings retain cognitive, cyclomatic, and statement values

### Requirement: Detailed and path views remain useful
`--all` SHALL show all useful findings and relevant resolved, unresolved,
ambiguous, advisory, ownership, incoming, and outgoing relationships without
showing healthy rows or internal processing facts. A selected path SHALL retain
relevant incoming and outgoing relationships even when one endpoint lies
outside that path. Human activity rows SHALL say `commit` or `commits`. Resolved
uses SHALL say `<source> → <target> · 1 import` with correct plural form;
ownership SHALL say `<source> owns <target>`; external, unresolved, and
ambiguous rows SHALL say `external`, `could not be matched`, and `matched more
than one file`. Primary/trusted SHALL be omitted. Non-primary roles and advisory
trust SHALL appear only when useful. Diff cycles SHALL use a short change label
followed by a closed arrow path. Edge changes SHALL use an arrow or ownership
row with added or removed wording.
Every coupling row shown in `--all` or a path view SHALL retain shared commits,
union commits, similarity, and end with `code dependency exists` or `no code
dependency`. A Worse coupling comparison SHALL say `<left> ↔ <right> now change
together without a code dependency`. A Better comparison SHALL say `<left> ↔
<right> no longer change together without a code dependency`. Human output
SHALL NOT expose the internal coupling comparison label.

#### Scenario: User requests all detail
- **WHEN** `--all` is supplied
- **THEN** count limits are removed for useful findings and relationships while healthy rows and processing facts remain absent

#### Scenario: User drills into a path
- **WHEN** a relationship crosses the selected path boundary
- **THEN** the relevant incoming or outgoing relationship remains visible

#### Scenario: Coupling detail and diff outcomes are shown
- **WHEN** a detailed or path view contains explained or unexplained coupling and a diff adds or removes its finding
- **THEN** each row retains exact commit evidence and dependency state while the diff names both operands and the direct outcome

### Requirement: Human diagnostics are grouped and simple
Incomplete history SHALL be written as `History is incomplete.` Rename gaps
SHALL be written as `Some renamed files could not be matched.` Unresolved and
ambiguous imports SHALL use correct singular or plural sentences such as `3
imports could not be matched.` and `1 import matched more than one file.`
Repeated parser and file warnings SHALL be grouped by default, with file detail
available in `--all` or a path view.

#### Scenario: Several files share a warning kind
- **WHEN** default output is rendered
- **THEN** one warning summary appears instead of one line per file

#### Scenario: Detailed warning context is requested
- **WHEN** the user supplies `--all` or selects an affected path
- **THEN** relevant file details appear without repeating the summary text for every fact

### Requirement: Human command text uses the same simple language
The system SHALL use short user-facing language for CLI help, argument errors,
runtime errors, terminal labels, and README examples and SHALL avoid internal report,
composition, stream, and mapping terms. The CLI SHALL retain existing exit-code
and stdout/stderr behavior.

#### Scenario: Help and failures are inspected
- **WHEN** help, an argument failure, and a runtime failure are invoked
- **THEN** each uses simple actionable text on the existing stream with the existing status

### Requirement: Machine and analysis interfaces do not change
The system SHALL preserve JSON version 3 bytes, report facts, finding rank,
analysis policy, exit codes, serial and automatic behavior, live work counts,
allocations, and measured resource behavior unchanged through terminal simplification.

#### Scenario: Terminal snapshots change
- **WHEN** the same generated fixtures are rendered as JSON and terminal output
- **THEN** only human terminal bytes differ and every machine, analysis, and resource check still passes

