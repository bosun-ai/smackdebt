## MODIFIED Requirements

### Requirement: Unexplained recurring coupling is a Watch finding
The system SHALL create a Watch finding only when a pair has at least three
shared commits, Jaccard similarity of at least 0.20, sufficient history, no
ancestor-descendant relationship between its endpoints, and no trusted eligible
`uses` relation in either direction. Trusted eligible `uses` SHALL mean parsed
`uses` from primary, test, example, or benchmark source, independently of whether
that relation enters an architecture verdict graph. A `uses` relation resolved
through a unique manifest-name match SHALL explain a pair exactly as a
path-resolved relation does. Weaker observations SHALL remain visible in JSON and
`--all` and SHALL NOT appear as default findings.

#### Scenario: A pair meets both thresholds
- **WHEN** a pair has 3 shared commits, 15 union commits, sufficient history, and no explaining use
- **THEN** one Watch finding exposes the exact operands and absent relationship

#### Scenario: A pair misses one threshold
- **WHEN** a pair has two shared commits or similarity below 0.20
- **THEN** it is absent from default findings but remains available in JSON and `--all`

#### Scenario: Trusted uses explain the pair
- **WHEN** an eligible parsed uses relation exists in either direction
- **THEN** coupling remains descriptive and creates no finding

#### Scenario: A test-role dependency explains the pair
- **WHEN** two packages are linked only by a trusted test-role `uses` relation, such as a dev-dependency import in a test file or in a `#[cfg(test)]` module
- **THEN** their coupling is explained, no Watch finding is created, and no output claims `no code dependency` for that pair, even though that relation never enters an architecture verdict graph

#### Scenario: A manifest-name edge explains the pair
- **WHEN** two workspace packages import each other only by declared manifest name
- **THEN** their coupling is explained, no Watch finding is created, and no output claims `no code dependency` for that pair
