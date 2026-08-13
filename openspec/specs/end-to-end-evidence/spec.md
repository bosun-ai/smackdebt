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

The system SHALL maintain public generated repositories with hand-calculated
facts for every supported language, static architecture, evolutionary history,
worktree comparison, and recoverable coverage failure.

#### Scenario: All-language analysis runs

- **WHEN** the all-language repository is analyzed
- **THEN** every listed supported language has nested-unit evidence for
  cognitive complexity, cyclomatic complexity, and exclusive logical lines
- **AND** exact values match its fact manifest

#### Scenario: Unified analysis runs

- **WHEN** a fixture contains source debt, a static cycle, churn, and unexplained
  change coupling
- **THEN** one report presents separate code, architecture, and evolution facts
- **AND** JSON links those facts through typed indexes without duplicate owned
  findings

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

The system SHALL compare committed terminal bytes at widths 120, 80, and 50 and
SHALL prove that forced color changes only ANSI styling, not visible text.

#### Scenario: Terminal width changes

- **WHEN** the same detailed report is rendered at each required width
- **THEN** every result exactly matches its committed expected bytes
- **AND** no content is lost merely because the terminal is narrow

#### Scenario: Color is forced

- **WHEN** color-disabled and forced-color results are compared after removing
  ANSI sequences
- **THEN** their visible text is byte-for-byte equal

### Requirement: JSON evidence validates structure and meaning

The system SHALL validate every acceptance JSON result against the committed
version-2 schema, assert hand-calculated values, and compare exact bytes.

#### Scenario: A JSON codebase report is produced

- **WHEN** unified analysis completes in JSON mode
- **THEN** schema validation passes before snapshot comparison
- **AND** focused assertions prove metric, graph, history, finding, diagnostic,
  and coverage values
- **AND** the complete bytes match the committed result

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

The system SHALL expose test-only counts for inventory, reads, Git processes,
parser visits, algorithm passes, and rendering without changing normal output.

#### Scenario: JSON renders an existing report

- **WHEN** instrumented JSON output receives the completed borrowed report
- **THEN** rendering starts no discovery, source read, Git process, parser visit,
  or analysis pass

#### Scenario: A worktree diff reads repository state

- **WHEN** the generated mixed-change fixture runs
- **THEN** measured source reads, object reads, and Git process counts match the
  applicable performance contracts

### Requirement: Installation works outside the source workspace

The system SHALL install locked local crates into a temporary prefix and run the
installed command from a generated repository outside the source workspace.

#### Scenario: Installed smoke test runs

- **WHEN** the release smoke test invokes the installed command
- **THEN** help and version succeed
- **AND** one terminal and one JSON report pass status, stream, schema, and
  semantic assertions

