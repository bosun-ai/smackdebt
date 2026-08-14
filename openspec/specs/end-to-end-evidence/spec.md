# end-to-end-evidence Specification

## Purpose
TBD - created by archiving change prove-unified-analysis. Update Purpose after archive.
## Requirements
### Requirement: Acceptance tests invoke the real CLI process

The system SHALL test public behavior by starting the built command with an
explicit working directory, arguments, environment, terminal settings, and
worker policy and by asserting status, stdout, and stderr bytes.

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
The system SHALL compare committed exact terminal bytes for codebase, diff,
package, directory, and file views at widths 120, 80, and 50. It SHALL prove
exact glyph code points and one-cell width, rejection of U+EC3F, alignment and
truncation, `--color always`, `--color never`, `NO_COLOR`, redirect behavior,
glyph-only ANSI placement, and equality after ANSI stripping.

#### Scenario: Terminal width changes
- **WHEN** each named report view is rendered at each required width
- **THEN** every result exactly matches reviewed bytes, preserves relevant facts, recognizable locations, glyphs, and commands, and every ANSI-stripped line fits the requested Unicode display width
- **AND** direct writer instrumentation records zero unexpected-line safety shortenings for the reviewed 50-column reports and deliberate long-identity fixture, while a synthetic overflow records use of that safety path

#### Scenario: Styling policy changes
- **WHEN** forced, disabled, automatic, `NO_COLOR`, and redirected modes are compared
- **THEN** only the required glyphs receive the exact colors and stripping ANSI reproduces plain bytes

#### Scenario: Glyph vocabulary is audited
- **WHEN** public results are scanned by scalar value and display width
- **THEN** every required glyph is exact and one cell, U+EC3F and redundant status words are absent, and Changed text stays normal

### Requirement: JSON evidence validates structure and meaning
Every acceptance JSON result SHALL validate against the version-3 schema and
pass semantic, index, privacy, and exact-byte checks from the same invocation
before its performance sample is accepted.

#### Scenario: A recovered High fact is emitted
- **WHEN** JSON validation runs
- **THEN** the advisory fact remains present and no health, graph-verdict, coupling-finding, or diff-verdict index refers to it

#### Scenario: A private value reaches output
- **WHEN** output contains source, an absolute private path, Git identity, or private history detail
- **THEN** privacy validation fails before evidence approval

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
Exact acceptance SHALL cover codebase, diff, package, directory, and file flows
for section relevance, five-area and three-finding limits, rank, architecture
witnesses, actionable history, detailed and path relationships, grouped
warnings, simple help and errors, commit singular/plural labels, direct
relationship wording, closed diff cycle paths, readable edge changes, and the
glyph-plus-command discover line. It SHALL also cover explained and weak
coupling evidence in detailed and path views plus direct introduced and removed
coupling outcomes in diff output.
JSON, status, stderr, ranking, serial/automatic bytes, work totals, analysis,
allocation, and performance evidence SHALL prove unchanged behavior.

#### Scenario: Human output is audited
- **WHEN** snapshots, help, errors, warnings, and README examples are checked
- **THEN** required sections and simple phrases appear while removed labels, severity words, processing facts, and forbidden phrases do not

#### Scenario: Machine and analysis output is audited
- **WHEN** revised terminal flows run through the complete public matrix
- **THEN** JSON bytes, exit behavior, rank, work counts, analysis facts, allocations, and resource evidence remain unchanged

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

