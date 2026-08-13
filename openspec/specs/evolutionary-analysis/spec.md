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

The system SHALL count file touches, package touches, textual additions, and
textual deletions without double-counting a package within one commit.

#### Scenario: One commit changes several files in one package

- **WHEN** three selected files in one package change in the same commit
- **THEN** each file receives one touch
- **AND** the package receives one touch
- **AND** the package receives the sum of the files' textual churn

### Requirement: Package change coupling is explainable

The system SHALL count one shared change per unordered package pair per commit,
retain pairs with at least two shared commits, and expose Jaccard numerator and
denominator values.

#### Scenario: Two packages repeatedly change together

- **WHEN** packages share three commits and six commits touch either package
- **THEN** their retained coupling has shared count 3, union count 6, and
  similarity 0.5

#### Scenario: A large commit changes a package more than once

- **WHEN** one commit changes several files in each of two packages
- **THEN** that commit adds exactly one shared change to their pair

### Requirement: Unexplained recurring coupling is a Watch finding

The system SHALL create a Watch architecture finding for a retained
cross-package coupling pair only when no static package dependency exists in
either direction.

#### Scenario: Repeated coupling lacks a static edge

- **WHEN** two packages share at least two commits and have no static edge in
  either direction
- **THEN** one Watch finding explains both packages, the shared and union
  counts, the similarity, and the absent static relationship

#### Scenario: A static edge explains the relationship

- **WHEN** a retained coupling pair has a static dependency in either direction
- **THEN** coupling remains descriptive and creates no coupling finding

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

