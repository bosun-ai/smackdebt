# end-to-end-evidence Specification

## Purpose
TBD - created by archiving change prove-unified-analysis. Update Purpose after archive.
## Requirements
### Requirement: Acceptance tests invoke the real CLI process

The system SHALL test public behavior by starting the built command with an
explicit working directory, arguments, environment, terminal settings, and
worker policy and by asserting status, stdout, and stderr bytes.

The child environment SHALL be hermetic: acceptance invocations and fixture
git commands SHALL pin the home directory, the user configuration directory,
and the global and system git configuration to harness-owned locations
through one shared helper, so no machine-level ignore file or git setting can
change evidence bytes. Global-gitignore behavior SHALL have positive evidence:
a harness-written global ignore file provably excludes a fixture file.

#### Scenario: A successful codebase report runs

- **WHEN** the acceptance harness invokes the built command in a generated
  repository
- **THEN** the process exits with the documented status
- **AND** stdout exactly matches the expected report
- **AND** stderr is empty

#### Scenario: Invocation fails before a report exists

- **WHEN** the command receives an invalid ref or invalid argument
- **THEN** it exits with the documented nonzero status
- **AND** the product error appears on stderr
- **AND** stdout contains no partial report

#### Scenario: The developer machine has a global gitignore

- **WHEN** the same acceptance suite runs on machines with different user
  ignore files and git configuration
- **THEN** every asserted byte is identical because the child environment is
  pinned

#### Scenario: A pinned global ignore is honored

- **WHEN** the harness writes a global ignore pattern into the pinned
  configuration home
- **THEN** the matching fixture file is excluded from the report

### Requirement: Generated fixtures cover all analysis families
Public generated repositories SHALL provide hand-calculated facts for six
roles, every precedence level, same-level conflict, parsed, recovered and failed
source, stable packages, both relation kinds, history fields, coupling threshold
edges, exact ranking, root labels, architecture witnesses, and de-duplication.

#### Scenario: Signal-quality fixture runs
- **WHEN** the real CLI analyzes the fixture
- **THEN** its fact manifest proves every expected role, trust, package, relation, history, finding, diagnostic, and coverage fact

#### Scenario: Role rules conflict
- **WHEN** two same-level rules assign different roles
- **THEN** exact acceptance proves exit 2, empty stdout, and the reviewed stderr

### Requirement: The public command matrix covers documented flows

The system SHALL test default, detailed, path-selected, codebase, ref-diff, and
worktree-diff flows across healthy, Watch, High, improved, worsened, changed,
partial-coverage, and fatal outcomes.

#### Scenario: A worktree contains mixed changes

- **WHEN** modified, renamed, deleted, and untracked source changes affect code,
  dependencies, cycles, and an existing coupling explanation
- **THEN** the worktree-diff result reports each supported outcome once
- **AND** status and streams match the documented policy

#### Scenario: Detail selection changes presentation

- **WHEN** the same report is requested for a file, directory, and package
- **THEN** each view includes relevant incoming architecture pressure and
  evolutionary context
- **AND** unrelated detail is omitted from terminal output
- **AND** analysis is not rerun for rendering

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

### Requirement: JSON evidence validates structure and meaning
Every acceptance JSON result SHALL validate against the version-4 schema and pass semantic,
index, privacy, and exact-byte checks from the same invocation before its
performance sample is accepted. Validation SHALL prove that the denormalized
head agrees with the tables it duplicates, that hotspots, size findings, orphan
files, finding kind discriminators, history window fields, and `manifest_name`
are present and consistent, and that no floating-point number appears anywhere
in the object.

#### Scenario: A recovered High fact is emitted
- **WHEN** JSON validation runs
- **THEN** the advisory fact remains present and no health, graph-verdict, coupling-finding, or diff-verdict index refers to it

#### Scenario: A private value reaches output
- **WHEN** output contains source, an absolute private path, Git identity, or private history detail
- **THEN** privacy validation fails before evidence approval

#### Scenario: The head disagrees with a table
- **WHEN** a summary count or worst entry differs from the table facts it duplicates
- **THEN** validation fails before exact-byte approval

#### Scenario: A float is serialized
- **WHEN** any value in the object is a floating-point number
- **THEN** validation fails before exact-byte approval

### Requirement: Serial and parallel public output is identical

The system SHALL produce byte-for-byte identical terminal and JSON output for
one-worker and automatic-parallel complete analysis.

#### Scenario: Worker policy changes

- **WHEN** the same generated repository and invocation run with one worker and
  automatic parallelism
- **THEN** status, stdout, and stderr are byte-for-byte identical

### Requirement: Golden evidence changes explicitly

The system SHALL keep normal tests read-only and SHALL update selected committed
results only through a named developer command that reports every changed file.

#### Scenario: CI finds output drift

- **WHEN** actual output differs from committed evidence
- **THEN** the test fails with a reviewable difference
- **AND** CI does not rewrite the expected file

### Requirement: Composition work is observable in complete flows
Feature-gated live evidence SHALL count inventory visits, reads, Git processes,
parser visits, algorithm passes, and renderer entry at their actual seams with
no evidence-only API or atomic cost in normal builds.

#### Scenario: Rendering starts from one completed report
- **WHEN** live snapshots are captured immediately before and after output
- **THEN** the delta has one renderer entry and zero inventory, read, Git, parser, and algorithm events

#### Scenario: Real work occurs after a snapshot
- **WHEN** one counted seam runs after an earlier live snapshot
- **THEN** a later snapshot changes and cannot be replaced by a frozen report value

### Requirement: Installation works outside the source workspace

The system SHALL install locked local crates into a temporary prefix and run the
installed command from a generated repository outside the source workspace.

#### Scenario: Installed smoke test runs

- **WHEN** the release smoke test invokes the installed command
- **THEN** help and version succeed
- **AND** one terminal and one JSON report pass status, stream, schema, and
  semantic assertions

### Requirement: Public command evidence covers every revised decision
Exact acceptance SHALL cover codebase, diff, package, directory, and file flows for the
verdict block, the frozen tier sentences, word-labeled counts including zero
counts, the family named in a diff verdict, the clean-diff verdict-only view,
the five-area limit and each ladder rung of the problem budget, hot evidence,
ranked card order, stacked cycle witnesses, stable-dependency card evidence, one
card per coupling pair, knowledge-concentration card evidence without identity,
grouped warnings, actionable diff findings, the `next: smackdebt <path>`
discover line, and `--all` as all useful debt including `detail` cards and
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

### Requirement: Serial and automatic flows preserve bytes and work
Every named public flow SHALL run with one worker and automatic parallelism.
Status and bytes SHALL match, and inventory, read, Git, parser, algorithm, and
renderer totals SHALL equal the exact manifest values for that flow.

#### Scenario: Worker policy changes
- **WHEN** the same fixture command runs under both policies
- **THEN** terminal and JSON bytes and exact work totals agree

### Requirement: Three workload families have privacy-safe acceptance
After public proof passes, aggregate read-only review SHALL cover self, a
private mixed application, and a private Rust workspace. Committed evidence
SHALL name only workload family and outcome categories and SHALL contain no
private path, source, Git identity, history, or raw terminal output.

#### Scenario: Self is reviewed
- **WHEN** default output is inspected
- **THEN** it leads with important debt and omits empty optional sections

#### Scenario: Private mixed application is reviewed
- **WHEN** default output is inspected
- **THEN** it leads with primary application findings and generated Rails schema remains outside default debt

#### Scenario: Private Rust workspace is reviewed
- **WHEN** default output is inspected
- **THEN** it leads with useful Rust findings and weak history and graph facts are absent

### Requirement: Workspace resolution has generated fixture evidence
Public generated repositories SHALL include a workspace fixture per supported
manifest kind — Cargo with a `[lib] name` override, npm with a scoped name,
Python `pyproject.toml`, and a gemspec — whose cross-package references are
written against declared names rather than paths. Exact acceptance SHALL prove
that those references become internal package `uses` edges, that package degree
and at least one package cycle appear only because of manifest-name resolution,
that a shadowed duplicate manifest name stays ambiguous with its diagnostic,
that a Rust `pub use a::b;` yields no `pub` dependency target, that one package
pair yields exactly one coupling row, that no ancestor-descendant pair exists,
and that no output claims `no code dependency` for a pair connected by a
manifest-name edge. JSON version 3 structure SHALL be proven unchanged while its
values move, and serial and parallel runs SHALL remain byte-identical.

#### Scenario: A workspace fixture is analyzed
- **WHEN** the real CLI analyzes a workspace fixture whose packages import each other by declared name
- **THEN** the result contains internal package edges, exact degree facts, and no external classification for those references

#### Scenario: A duplicate manifest name is present
- **WHEN** two fixture packages declare the same name and a third imports it
- **THEN** the reference is ambiguous with a retained diagnostic and no internal edge is created

#### Scenario: Coupling evidence is audited
- **WHEN** a fixture produces coupling for a package pair and for a scope pair where one contains the other
- **THEN** exactly one row exists for the package pair and no row exists for the containing pair

### Requirement: New debt signals have generated fixture evidence
Public generated repositories SHALL provide hand-calculated facts for hotspots
at the minimum-touch boundary, rank ordering where hot and role class decide the
result, stable-dependency violations at the reference minimum and at equal
integer cross products, knowledge concentration at the 10-commit and 90%-share
boundaries, file and container size at their exact thresholds, orphan files
including an exempt entry file, and history coverage window fields for a
windowed and an unwindowed run. Facts a version 3 document and the terminal
cannot express SHALL be proven against the composed report from the crate that
owns composition, and SHALL move to command evidence when
`adopt-report-schema-v4` serializes those tables. Evidence SHALL prove that no contributor
identity appears in any output, that terminal sections, labels, and vocabulary
are unchanged and terminal bytes differ only where the new rank keys reorder
findings, that JSON version 3 shape is unchanged, and that serial and parallel
runs stay byte-identical
with unchanged inventory, read, parser, and Git process totals.

#### Scenario: Signal fixtures run
- **WHEN** the real CLI analyzes the deepened-signal fixtures
- **THEN** every expected rank order, windowed history value, and privacy result matches its hand-calculated value

#### Scenario: A signal reaches no serialized surface
- **WHEN** hotspots, stable-dependency findings, knowledge concentration, size findings, orphan facts, or window coverage fields cannot appear in a version 3 document or the terminal
- **THEN** generated repositories prove each expected value against the composed report until a later change serializes the table

#### Scenario: Privacy is audited
- **WHEN** output is scanned for the fixtures' known author names and addresses
- **THEN** none appear and concentration evidence contains counts only

#### Scenario: Unchanged interfaces are audited
- **WHEN** the same fixtures render terminal and JSON output before and after this change
- **THEN** terminal sections, labels, and vocabulary are unchanged, JSON version 3 structure is unchanged, and every differing terminal byte is explained by the new rank keys

### Requirement: Verdict policy has exact boundary evidence
Public generated repositories SHALL provide hand-calculated facts for every
codebase tier including both permille boundaries at exactly 1% and exactly 5%,
both architecture escalation floors, a floor that must not lower an already
higher tier, all four diff tiers, the contradiction case where no source
comparison moves debt and a package cycle is introduced, a diff whose only
changes are healthy additions, and worst-offender selection for the hotspot
reason, the complexity reason, the cycle fallback, and the absent case. Evidence
SHALL prove that scope verdicts differ from the root verdict where the facts
differ, that a duplicate debt-diff identity fails index integrity, that no
floating-point value participates in tier selection, and that serial and
parallel runs produce identical verdicts and selections.

#### Scenario: Tier fixtures run
- **WHEN** the real CLI analyzes the verdict fixtures
- **THEN** every tier id, sentence, count, and worst offender matches its hand-calculated value

#### Scenario: The contradiction case runs
- **WHEN** a diff introduces a package cycle without moving any source comparison
- **THEN** the diff verdict is `worse` and its facts name the architecture family

#### Scenario: Index integrity is audited
- **WHEN** a scope's debt-diff selection contains a duplicate identity
- **THEN** acceptance fails before exact-byte approval

### Requirement: The ratchet gate has exact acceptance evidence
Black-box evidence SHALL cover the gate: a clean gate exiting 0, a regressed
gate exiting 3 while naming the offending rows, a missing baseline exiting 2
with the exact documented stderr bytes, `--update` proven byte-stable and
idempotent, and gate `--json` validated against its checked schema.

#### Scenario: The gate regresses
- **WHEN** the acceptance fixture's debt exceeds its committed baseline
- **THEN** the real CLI exits 3 and stdout names each regressed row exactly

#### Scenario: The gate JSON is validated
- **WHEN** a gate result is emitted as JSON in acceptance
- **THEN** it validates against the gate schema before exact-byte approval

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
pattern and exact acceptance SHALL prove that each one is present: one file with
three High findings producing one card rather than three rows, one card per
strongly connected component carrying the existing stacked witness, a
`god_file`, a `hub`, a `hot_mess` that agrees with the `hot_and_complex`
worst-offender reason, one card per coupling, concentration, and
stable-dependency finding keeping that finding's exact operands, a `measured`
card whose head equals the finding row head it replaces, and a `detail` card
absent by default and present under `--all`.

Threshold-boundary evidence — a `god_file` at each arm of its rule with one unit
either side of each threshold, a `hub` at the degree boundary and at the exact
median multiple — SHALL be pure policy tests beside the rules rather than
generated repository fixtures. A boundary is a property of the rule, and a
fixture that pins one is a repository built to hold a number: it states the
threshold twice, moves whenever review moves the constant, and proves nothing
the pure test does not prove more directly. The black-box obligation for those
patterns is presence, not the boundary.

Evidence SHALL prove the ranked order across patterns, that no finding is
claimed by two cards, that no retained finding is claimed by none, and that
serial and parallel runs produce identical cards in identical order.

Evidence SHALL also cover the content that used to be dropped when no card
claimed it: a file whose whole debt is advisory or non-primary, and a file whose
only evidence is its size, each reaching a `detail` card that the default view
withholds and `--all` states with the size finding's subject and measured value.

#### Scenario: Card fixtures run
- **WHEN** the real CLI analyzes the card fixtures
- **THEN** every expected pattern, rating, evidence value, and rank position matches its hand-calculated value

#### Scenario: A detail card is requested
- **WHEN** the same fixture is rendered by default and with `--all`
- **THEN** the `detail` card is absent from the first and present in the second

#### Scenario: Advisory and size-only debt is requested
- **WHEN** a fixture holding a recovered file and an oversized file with no unit debt is rendered with `--all`
- **THEN** both reach a card, the advisory card shows its advisory trust, and the size card shows the size finding's subject and measured value

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

