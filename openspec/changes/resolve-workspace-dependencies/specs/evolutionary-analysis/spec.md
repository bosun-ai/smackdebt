## MODIFIED Requirements

### Requirement: Package change coupling is explainable
The system SHALL retain left and right package IDs, `shared_commits`,
`union_commits`, and Jaccard similarity for each retained unordered pair. Each
unordered package pair SHALL produce exactly one retained coupling row;
SourceRole and trust variants SHALL be aggregated into that row rather than
emitted as sibling rows, and per-role evidence SHALL remain in package history
rows. Eligible parsed roles SHALL feed one package-pair aggregate used for
findings, so fixture, generated, recovered, and failed history cannot change
finding operands.

A pair SHALL be excluded before similarity is computed when one endpoint scope
is an ancestor of the other, because shared commits between a scope and its own
descendant are structural rather than evidence of hidden coupling.

#### Scenario: Two packages change together
- **WHEN** packages share three commits and fifteen commits touch either package
- **THEN** the row exposes shared count 3, union count 15, and similarity 0.20

#### Scenario: One commit changes several files per package
- **WHEN** the same pair occurs several times inside one commit
- **THEN** that commit adds one shared change to the pair

#### Scenario: One pair has several role and trust variants
- **WHEN** a package pair is touched by primary, test, and generated source
- **THEN** exactly one coupling row exists for that pair with one set of operands, and no view can print the same pair with different numbers

#### Scenario: One scope contains the other
- **WHEN** the repository root scope and one of its packages change in the same commits
- **THEN** no coupling row and no coupling finding exists for that pair

### Requirement: Unexplained recurring coupling is a Watch finding
The system SHALL create a Watch finding only when a pair has at least three
shared commits, Jaccard similarity of at least 0.20, sufficient history, no
ancestor-descendant relationship between its endpoints, and no trusted eligible
`uses` relation in either direction. A `uses` relation resolved through a unique
manifest-name match SHALL explain a pair exactly as a path-resolved relation
does. Weaker observations SHALL remain visible in JSON and `--all` and SHALL NOT
appear as default findings.

#### Scenario: A pair meets both thresholds
- **WHEN** a pair has 3 shared commits, 15 union commits, sufficient history, and no explaining use
- **THEN** one Watch finding exposes the exact operands and absent relationship

#### Scenario: A pair misses one threshold
- **WHEN** a pair has two shared commits or similarity below 0.20
- **THEN** it is absent from default findings but remains available in JSON and `--all`

#### Scenario: Trusted uses explain the pair
- **WHEN** an eligible parsed uses relation exists in either direction
- **THEN** coupling remains descriptive and creates no finding

#### Scenario: A manifest-name edge explains the pair
- **WHEN** two workspace packages import each other only by declared manifest name
- **THEN** their coupling is explained, no Watch finding is created, and no output claims `no code dependency` for that pair
