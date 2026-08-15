## MODIFIED Requirements

### Requirement: Unified analysis evidence precedes publication
Authoring all five v-next change packages before implementation begins SHALL be
allowed. Their implementation, review, and archive order SHALL be
`resolve-workspace-dependencies`, then `deepen-debt-signals`, then
`add-verdict-policy`, then `redesign-terminal-report`, then
`adopt-report-schema-v4`. The workspace SHALL remain private until every one of
those changes is implemented, reviewed, and archived and until the exact human
interface, machine contract, analysis behavior, public width and resource gates,
installed behavior, and three privacy-safe workload outcomes pass from their
reviewed implementation revisions. The `prepare-first-release` change SHALL
remain blocked until all five are archived.

Release review SHALL confirm that piped output contains no codepoint in
U+E000–U+F8FF, that every count is labeled with its word, that the three exact
input-failure messages are byte-correct, and that the README matches the
rendered report.

#### Scenario: The terminal redesign is reviewed but the schema change remains
- **WHEN** `redesign-terminal-report` passes but `adopt-report-schema-v4` is not archived
- **THEN** `prepare-first-release` remains blocked and publication evidence is not finalized

#### Scenario: Piped output leaks a private-use glyph
- **WHEN** any reviewed workload's piped output contains a codepoint in U+E000–U+F8FF
- **THEN** release evidence fails

#### Scenario: Implementation is reviewed but not committed
- **WHEN** local results look correct before the related implementation revision exists
- **THEN** clean release evidence is not recorded

#### Scenario: Reviewed implementation commit is clean
- **WHEN** every implementation and expectation change is committed
- **THEN** the release workflow records exact public profiles and aggregate workload outcomes from that clean HEAD
