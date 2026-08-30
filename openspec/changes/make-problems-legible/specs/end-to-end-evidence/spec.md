## MODIFIED Requirements

### Requirement: Terminal evidence is exact and stable
The system SHALL compare committed exact terminal bytes for codebase, diff, package,
directory, and file views at widths 120, 100, 80, and 50, and SHALL compare
piped bytes with terminal bytes for the same invocation. Evidence SHALL cover a
verdict block per codebase tier and per diff tier, word-labeled counts including
zero counts, a clean diff that prints the verdict line only, hot evidence with
its commit count, stacked cycle witnesses carried as problem-card evidence,
stable-dependency card evidence with its integer operands, grouped warnings, and
diff findings with `path:line`, before-and-after measurements, and human
identities.

Evidence SHALL prove that one codebase invocation states the same cards and the
same evidence items at every required width, so the slot budget never makes the
content of a report depend on the terminal it is read in.

It SHALL prove that piped output contains no codepoint in the range
U+E000–U+F8FF, that decorated output uses exact glyph code points of one display
cell placed adjacent to their words, that U+EC3F is absent, that `--color
always`, `--color never`, `NO_COLOR`, and redirect behavior are exact, and that
stripping ANSI reproduces the plain words. Every ANSI-stripped line SHALL fit
its requested Unicode display width, and no measurement, count, cycle witness,
history evidence value, dependency state, command, or identity SHALL be silently
clipped at any required width.

#### Scenario: Terminal width changes
- **WHEN** each named report view is rendered at every required width
- **THEN** every result exactly matches reviewed bytes, stacks rows that do not fit, and loses no fact

#### Scenario: The same report is read at two widths
- **WHEN** one codebase invocation is rendered at 50 and at 120 columns
- **THEN** both state the same cards and the same evidence items and differ only in row stacking

#### Scenario: Output is piped
- **WHEN** the same command is captured through a pipe and through a terminal
- **THEN** the piped bytes state every meaning in words, contain no codepoint in U+E000–U+F8FF, and differ from the terminal bytes only by decoration and ANSI

#### Scenario: A cycle witness does not fit
- **WHEN** a cycle card is rendered at 50 columns
- **THEN** its witness stacks across lines with no ellipsis and the full closure remains readable

### Requirement: Public command evidence covers every revised decision
Exact acceptance SHALL cover codebase, diff, package, directory, and file flows for the
verdict block, the frozen tier sentences, word-labeled counts including zero
counts, the family named in a diff verdict, the clean-diff verdict-only view,
the five-area limit and each ladder rung of the problem budget, hot evidence,
ranked card order, stacked cycle witnesses, stable-dependency card evidence, one
card per coupling pair, knowledge-concentration card evidence without identity,
grouped warnings, actionable diff findings, the `next: smackdebt <path>`
discover line, and `--all` as all useful debt including descriptive cards and
without raw edges, standard-library externals, churn dumps, cyclomatic-1 rows,
weak coupling, or healthy rows.

It SHALL cover full exact stderr bytes including one newline, required status,
and empty stdout for `smackdebt: path not found: <user-path>`, `smackdebt: Git
ref not found: <ref>`, and `smackdebt: --all cannot be used with --json`, with
no usage or help tail and no leaked absolute path, operating-system code, Git
command, status, fatal output, parser text, or process text.

The current JSON contract's existing members' bytes and report facts, status
classes, stream placement, the accepted finding rank, serial and automatic
bytes, work totals, analysis, allocation, and performance evidence SHALL prove
unchanged behavior. The additive problem table and the verdict share are the
only JSON members this change introduces, and no existing member SHALL be
removed or change meaning; evidence SHALL prove that too. Where this requirement
says rank, it means the accepted finding rank, which is unchanged; the problem
rank is a separate order over the new table.

#### Scenario: Human output is audited
- **WHEN** snapshots, help, errors, warnings, and README examples are checked
- **THEN** the word vocabulary, verdict block, and labeled counts appear while positional counts, omitted zero counts, ellipsis-truncated witnesses, generated internal identities, raw edges, and leaked implementation diagnostics do not

#### Scenario: An input failure occurs
- **WHEN** each of the three failures is invoked
- **THEN** status, empty stdout, and one exact newline-terminated stderr line match the required bytes with no usage or help tail

#### Scenario: Machine and analysis output is audited
- **WHEN** redesigned terminal flows run through the complete public matrix
- **THEN** every JSON member that existed before this change keeps its bytes and report facts, and exit classes, streams, the accepted finding rank, work counts, allocations, and resource evidence remain unchanged

#### Scenario: The added JSON members are audited
- **WHEN** a result from before this change is compared with a result from after it
- **THEN** the only differences are the added problem table and the verdict share, and no existing member was removed or redefined

## ADDED Requirements

### Requirement: The one-screen budget has exact evidence at every scope
Black-box evidence SHALL prove that a default codebase view fits the budget at
every scope. At the redirected default width the problem section body SHALL be
at most the budget in lines for a repository, package, directory, and file view
of a repository large enough to exceed it, and the view the report's own `next:`
line proposes SHALL itself fit the budget, because the reported defect was
exactly that following the tool's own advice produced a longer report.

Evidence SHALL cover each ladder rung by a scope holding that many cards, a
scope holding more cards than the largest rung proving the cut and the absence
of bookkeeping rows, `--top 10` showing ten cards, `--top` above the card count
showing every card without filler, and a file scope showing complete evidence
without the ladder.

#### Scenario: A large directory is the selected scope
- **WHEN** a directory scope holding hundreds of files is rendered by default
- **THEN** its problem section body fits the budget and states ranked, named cards

#### Scenario: The report's own advice is followed
- **WHEN** the path from the `next:` line is used as the next invocation
- **THEN** that view also fits the budget

#### Scenario: A middle limit is requested
- **WHEN** `--top 10` is supplied over a scope holding more than ten cards
- **THEN** exactly ten cards are shown with the evidence allowance of the rung ten selects

### Requirement: Problem cards have exact black-box evidence
Generated repositories SHALL provide hand-calculated facts for each frozen
pattern and exact acceptance SHALL prove them: one file with three High findings
producing one card rather than three rows, one card per strongly connected
component carrying the existing stacked witness, a `god_file` at each arm of its
rule and one unit either side of each threshold, a `hub` at the degree boundary
and at the exact median multiple in a multi-package fixture, a `hot_mess` that
agrees with the `hot_and_complex` worst-offender reason, one card per coupling,
concentration, and stable-dependency finding keeping that finding's exact
operands, a `measured` card whose head equals the finding row head it replaces,
and a descriptive card absent by default and present under `--all`.

Evidence SHALL prove the ranked order across patterns, that no finding is
claimed by two cards, and that serial and parallel runs produce identical cards
in identical order.

#### Scenario: Card fixtures run
- **WHEN** the real CLI analyzes the card fixtures
- **THEN** every expected pattern, rating, evidence value, and rank position matches its hand-calculated value

#### Scenario: A descriptive card is requested
- **WHEN** the same fixture is rendered by default and with `--all`
- **THEN** the descriptive card is absent from the first and present in the second

#### Scenario: Worker policy changes
- **WHEN** a card fixture runs with one worker and with automatic parallelism
- **THEN** the cards and their order are byte-identical

### Requirement: Human views carry no dependency edge rows
Every committed human view, at every scope and every detail level, SHALL be
proven free of dependency-edge rows: no row heading two repository file paths
with an arrow or with ` owns `, no line stating the single-reference import fact
`· 1 import`, and no arrow joining two repository file paths outside a cycle
witness. The check SHALL be an invariant over every committed terminal snapshot
rather than an assertion on selected cases, so a future view cannot reintroduce
the rows quietly.

The check SHALL be written against file paths, because two package identities
joined by an arrow are not an edge row: an `unstable_dependency` card head keeps
the accepted `<source> → <target>` package wording, and its reference-count
evidence is never the single-reference form, since a stable-dependency finding
requires at least two references.

Evidence SHALL prove that unresolved and ambiguous rows appear at a file scope
and under `--all`, and that at every other scope the grouped warning sentence is
their whole terminal presence.

#### Scenario: Every snapshot is scanned
- **WHEN** the committed terminal snapshots are scanned as a set
- **THEN** none heads two repository file paths with an arrow or with ` owns `, none states `· 1 import`, and none joins two repository file paths with an arrow outside a cycle witness

#### Scenario: An unfollowed import is inspected
- **WHEN** the same fixture is rendered at a directory scope, at a file scope, and with `--all`
- **THEN** only the file scope and the `--all` run print per-import rows and all three print the grouped sentence

### Requirement: The repository frame has exact evidence
Exact acceptance SHALL prove the repository-share sentence: its exact bytes at a
package scope and at a directory scope of a repository holding High debt, its
absence at the repository root, and its absence when the repository holds no
High debt. The serialized share SHALL carry the same counts and sentence bytes
the terminal prints.

#### Scenario: A package view is rendered
- **WHEN** a package scope of a repository with High debt is rendered
- **THEN** the verdict block contains the exact share sentence with both counts

#### Scenario: The root view is rendered
- **WHEN** no path is selected
- **THEN** no share sentence appears in the terminal and no share member appears in JSON

### Requirement: Dependency specifiers have single-line evidence
Exact acceptance SHALL prove that no retained resolution diagnostic target
contains a newline, carriage return, or tab in any result, using a fixture whose
source contains a dynamic import written across several lines with a template
literal. Terminal evidence SHALL prove the same target renders as one wrapped
row rather than raw source.

#### Scenario: A multi-line import is analyzed
- **WHEN** the fixture containing a three-line dynamic import is analyzed
- **THEN** no `resolution_diagnostics` target contains a newline and the terminal prints the specifier on wrapped rows without raw source
