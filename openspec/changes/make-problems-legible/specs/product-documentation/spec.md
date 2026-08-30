## MODIFIED Requirements

### Requirement: Documentation covers the unified result
The README SHALL show how one command answers code and architecture questions, opening
with the verdict block — scope, tier sentence, word-labeled counts, and worst
offender — followed by `AREAS`, `PROBLEMS`, and `WARNINGS` for a codebase
report, with empty optional sections omitted. It SHALL show that a diff report
keeps `AREAS`, `FINDINGS`, `ARCHITECTURE`, `HISTORY`, and `WARNINGS` this
release and SHALL say plainly that codebase and diff output speak different
vocabularies for one cycle rather than leaving the reader to guess. It SHALL
list the frozen codebase and diff tier ids with their sentences, state that a
clean diff prints the verdict line only, and state that every count is labeled
with its word including zero counts.

#### Scenario: A user reads the main examples
- **WHEN** the user follows documented default, path, and diff examples
- **THEN** the examples lead with the verdict, label every count, omit empty sections, and do not present a combined score

#### Scenario: A user compares a codebase and a diff report
- **WHEN** the user reads both documented examples
- **THEN** the README explains why one shows problem cards and the other still shows finding, architecture, and history sections

#### Scenario: A machine consumer reads the documentation
- **WHEN** an integration author decides what to key on
- **THEN** the README names the tier ids and the problem pattern ids as the stable vocabulary and points to JSON for complete data

### Requirement: README documents terminal presentation controls
Product documentation SHALL explain that severity, direction, and diagnostics are stated in
words, that glyphs and the tier bar are optional decoration resolved from
terminal detection, and that piped output is the same report in words with no
private-use codepoint. It SHALL explain `--color`, `NO_COLOR`, width behavior,
content-aware row stacking, `--top N` as a middle level of detail — problem
cards in a codebase report and ranked comparisons in a diff report — that
conflicts with `--json` and `--all`, `--all` as all useful debt including
descriptive problem cards, and JSON as the complete machine-readable view of
everything the terminal omits. It SHALL state that there is no icon option,
emoji mode, or theme setting.

It SHALL explain the one-screen budget: that a codebase view spends a fixed
number of slots on problems, that showing more problems means showing less
evidence for each, that the budget counts slots rather than rendered lines so
the same invocation states the same facts at every terminal width, and that a
narrow terminal therefore stacks rows and can run longer than one screen.

The previously documented commitment that there is no ASCII fallback SHALL be
removed, because undecorated words-only output is now the defined behavior for
non-terminal and color-disabled output.

#### Scenario: User runs Smackdebt in automation
- **WHEN** the user reads output guidance
- **THEN** the README explains word-only piped output, how to force or disable decoration and ANSI styling, and that JSON remains unstyled and complete

#### Scenario: User looks for the fallback statement
- **WHEN** the user reads presentation documentation
- **THEN** no claim that ASCII or plain fallback is unavailable appears

#### Scenario: User wants more problems without everything
- **WHEN** the user reads the detail documentation
- **THEN** the README explains `--top N`, that it counts cards in a codebase report, what it trades away, and its conflicts

#### Scenario: User asks why a narrow terminal scrolls
- **WHEN** the user reads the budget documentation
- **THEN** it explains that content is width-independent and that stacking, not extra content, makes a narrow view longer

### Requirement: README states JSON detail retention
The README SHALL state that JSON retains every Watch and High finding and all scope
summaries, while healthy units are represented through aggregate counts. It
SHALL also state that JSON is the complete view of every fact the terminal
omits, including raw dependency edges, external references, churn detail, weak
coupling, hotspots, size findings, orphan files, and every problem card
including descriptive ones.

It SHALL state that dependency edges appear in no human view at any scope or
detail level, that the terminal states a relationship only as aggregate problem
evidence such as a fan-in count or a cycle witness, and that the JSON relation
tables are therefore the only place to read the edges themselves.

#### Scenario: Integration author chooses JSON output
- **WHEN** an integration needs all debt findings
- **THEN** the README makes clear that JSON is complete for Watch and High findings but not a full healthy-unit index

#### Scenario: A user misses a row the terminal removed
- **WHEN** they look for a row that human output no longer prints
- **THEN** the README directs them to the version-4 JSON table that retains it

#### Scenario: A user looks for import rows
- **WHEN** they read the architecture or JSON documentation after their import rows disappeared
- **THEN** the README states that edges are JSON-only and shows the card evidence that replaced them

## ADDED Requirements

### Requirement: README explains problem cards
The README SHALL explain that a codebase report names problems rather than
listing measurements: that a problem card groups findings the report already
produced around one file, cycle, package, or package pair, that each finding
belongs to at most one card so a file with several findings is named once, and
that clustering invents no measurement, rating, or verdict.

It SHALL list the frozen pattern ids `god_file`, `hub`, `tangle`, `hot_mess`,
`shotgun_pair`, `bus_risk`, `unstable_dependency`, and `measured` beside the
words the terminal prints for each, explain in product language what evidence
each one needs, and publish the threshold behind every named pattern the way the
existing rating thresholds are published. It SHALL explain that `measured` is
the fallback so nothing rated disappears, that a widely imported file with no
debt of its own is a descriptive card shown only under `--all` and in JSON, and
that cards are ranked against each other so the first card is the problem to
look at first.

It SHALL document the repository-share sentence in a package, directory, or file
verdict, state that it is absent at the repository root, and explain that it
frames how much of the whole problem the selected scope holds.

#### Scenario: A user sees a named problem
- **WHEN** the user reads a card in their own report and looks it up
- **THEN** the README names the pattern, states the evidence it needs, and explains what to do about it

#### Scenario: A user misses their per-finding rows
- **WHEN** a user who knew the old finding list reads the new documentation
- **THEN** it explains that one card now claims that file's findings and that JSON retains each of them

#### Scenario: A user drills into a package
- **WHEN** the user reads the sub-scope example
- **THEN** the README explains the share sentence and why the repository root does not print one
