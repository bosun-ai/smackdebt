## MODIFIED Requirements

### Requirement: Package change coupling is explainable
The system SHALL retain left and right package IDs, `shared_commits`,
`union_commits`, and Jaccard similarity for each retained unordered pair. Each
source-derived observation SHALL retain SourceRole so fixture and generated
history can remain descriptive without affecting findings.

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

### Requirement: Churn and touches use exact aggregation rules
File and package history SHALL expose `touches`, `added_lines`, `deleted_lines`,
and `uncounted_changes`. A package SHALL receive at most one touch per commit,
textual lines SHALL be summed exactly, and binary changes SHALL increase
uncounted changes without invented line counts.

#### Scenario: One commit changes text and a binary file
- **WHEN** both changes belong to one package
- **THEN** the package gains one touch, exact textual lines, and one uncounted change

## ADDED Requirements

### Requirement: History coverage and concentration fields are exact
History coverage SHALL expose availability, revision, commit count, newest and
oldest timestamps, textual changes, uncounted changes, excluded paths, rename
gaps, and reason. Contributor concentration SHALL expose package ID,
contributor count, numerator, denominator, and ratio without identity.

#### Scenario: History is shallow
- **WHEN** only part of repository history is locally available
- **THEN** every field remains explicit and reason states incomplete history

### Requirement: Default history presentation does not repeat facts
Default terminal output SHALL present one package history row, coupling pair, or
contributor concentration at its nearest useful scope at most once. `--all` and
path drill MAY expand evidence but SHALL NOT duplicate an identical row within
one rendered view.

#### Scenario: A coupling finding appears with package history
- **WHEN** default output includes both sections
- **THEN** its shared and union operands are printed once rather than repeated in summary and detail
