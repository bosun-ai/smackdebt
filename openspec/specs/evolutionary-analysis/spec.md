# evolutionary-analysis Specification

## Purpose
TBD - created by archiving change add-evolutionary-architecture-analysis. Update Purpose after archive.
## Requirements
### Requirement: History is streamed once into compact facts

The system SHALL read locally available non-merge history through one streamed
Git process per report and retain only the facts required for aggregation.

#### Scenario: One report reads history

- **WHEN** a codebase report analyzes a repository with many commits
- **THEN** exactly one history process supplies commit, rename, churn, and
  normalized contributor facts
- **AND** full commit messages and source contents are not retained

#### Scenario: A binary change is present

- **WHEN** Git reports a changed file without textual line counts
- **THEN** the change contributes a file and package touch
- **AND** the system does not invent added or deleted line counts

### Requirement: Current file identity follows supported renames

The system SHALL attribute older history to a selected current file only when a
parsed rename chain establishes that identity.

#### Scenario: A file was renamed

- **WHEN** a selected current file has an unbroken rename chain
- **THEN** touches and textual churn from its earlier paths belong to that
  current file

#### Scenario: A rename chain cannot be trusted

- **WHEN** an earlier path cannot be linked uniquely to a selected current file
- **THEN** the system excludes that record from current file measurements
- **AND** history coverage reports the exclusion

### Requirement: Churn and touches use exact aggregation rules
File and package history SHALL expose `touches`, `added_lines`, `deleted_lines`,
and `uncounted_changes`. A package SHALL receive at most one touch per commit,
textual lines SHALL be summed exactly, and binary changes SHALL increase
uncounted changes without invented line counts. Rows SHALL retain SourceRole and
trust after aggregation. Package history SHALL keep separate evidence rows for
eligible parsed source and context source instead of merging their operands.

#### Scenario: One commit changes text and a binary file
- **WHEN** both changes belong to one package
- **THEN** the package gains one touch, exact textual lines, and one uncounted change

### Requirement: Package change coupling is explainable
The system SHALL retain left and right package IDs, `shared_commits`,
`union_commits`, and Jaccard similarity for each retained unordered pair. Each
descriptive source-derived pair SHALL retain the left and right SourceRole and
trust. Eligible parsed roles SHALL also feed one package-pair aggregate used
only for findings, so fixture, generated, recovered, and failed history cannot
change finding operands.

#### Scenario: Two packages change together
- **WHEN** packages share three commits and fifteen commits touch either package
- **THEN** the row exposes shared count 3, union count 15, and similarity 0.20

#### Scenario: One commit changes several files per package
- **WHEN** the same pair occurs several times inside one commit
- **THEN** that commit adds one shared change to the pair

### Requirement: Unexplained recurring coupling is a Watch finding
The system SHALL create a Watch finding only when a pair has at least three
shared commits, Jaccard similarity of at least 0.20, sufficient history, and no
trusted eligible `uses` relation in either direction. Weaker observations SHALL
remain visible in JSON and `--all` and SHALL NOT appear as default findings.

#### Scenario: A pair meets both thresholds
- **WHEN** a pair has 3 shared commits, 15 union commits, sufficient history, and no explaining use
- **THEN** one Watch finding exposes the exact operands and absent relationship

#### Scenario: A pair misses one threshold
- **WHEN** a pair has two shared commits or similarity below 0.20
- **THEN** it is absent from default findings but remains available in JSON and `--all`

#### Scenario: Trusted uses explain the pair
- **WHEN** an eligible parsed uses relation exists in either direction
- **THEN** coupling remains descriptive and creates no finding

### Requirement: Contributor concentration protects identity

The system SHALL report contributor count and top-contributor concentration
without retaining or emitting contributor identity.

#### Scenario: Several contributors touch one package

- **WHEN** normalized contributors have package touch counts 3, 2, and 1
- **THEN** the package reports contributor count 3, numerator 3, denominator 6,
  and concentration 0.5
- **AND** no name, address, raw author field, or internal contributor identifier
  enters terminal or JSON output

### Requirement: History limitations remain visible

The system SHALL preserve source and static architecture results when history is
unavailable or incomplete and SHALL attach an exact history status and
diagnostic.

#### Scenario: The selected directory has no Git history

- **WHEN** source analysis succeeds outside a Git repository
- **THEN** the report contains source and static architecture results
- **AND** evolution is unavailable with a diagnostic

#### Scenario: The repository is shallow

- **WHEN** only shallow local history is available
- **THEN** the report identifies the analyzed window as incomplete
- **AND** does not describe the values as complete repository history

### Requirement: Diff reports use history only as context

The system SHALL show existing history facts for changed files and packages
without treating those facts as before-and-after measurements.

#### Scenario: A worktree edits a high-churn package

- **WHEN** diff analysis changes source inside a package with retained history
- **THEN** the package's churn and coupling appear as context
- **AND** unchanged historical values are not labelled Better or Worse

#### Scenario: A worktree adds an explaining static edge

- **WHEN** a static edge introduced by the worktree explains an existing
  coupling finding
- **THEN** the evolutionary finding is reported as removed
- **AND** the historical coupling values remain unchanged

### Requirement: History coverage and concentration fields are exact
History coverage SHALL expose stream availability, revision, total streamed
commits, commits containing eligible current source, mapped eligible changes,
mapped context changes, newest and oldest timestamps, textual changes,
uncounted changes, excluded changes, rename gaps, and reason. A mapped fixture,
generated, recovered, or failed change is context rather than excluded.
Textual plus uncounted changes SHALL equal mapped eligible plus context changes,
and eligible commits SHALL NOT exceed streamed commits.

Contributor concentration SHALL expose package ID, SourceRole, trust,
contributor count, numerator, denominator, and ratio without identity. Eligible
and context contributors SHALL remain in separate rows so context contributors
cannot alter eligible top share.

#### Scenario: History is shallow
- **WHEN** only part of repository history is locally available
- **THEN** every field remains explicit and reason states incomplete history

#### Scenario: A complete stream has no eligible current source
- **WHEN** Git streaming completes but zero commits and changes map to eligible current source
- **THEN** history observations remain descriptive and no evolutionary finding is created

#### Scenario: Generated history dominates a package
- **WHEN** many generated contributors touch a package and few eligible parsed commits touch it
- **THEN** JSON and `--all` retain both evidence rows while default churn and top share use only the eligible row

### Requirement: Stream and mapping evidence are presented separately
Terminal history coverage SHALL state stream completeness separately from
eligible mapping counts and percentage. A complete Git stream alone SHALL NOT
be described as sufficient architecture evidence.

#### Scenario: Most observed changes are context or excluded
- **WHEN** Git streaming is complete and only some changes map to eligible current source
- **THEN** terminal output reports complete stream evidence and the eligible mapped count, total observed count, percentage, and eligible commit count separately

### Requirement: Default history presentation does not repeat facts
Default terminal output SHALL present one package history row, coupling pair, or
contributor concentration at its nearest useful scope at most once. `--all` and
path drill MAY expand evidence but SHALL NOT duplicate an identical row within
one rendered view.

#### Scenario: A coupling finding appears with package history
- **WHEN** default output includes both sections
- **THEN** its shared and union operands are printed once rather than repeated in summary and detail

