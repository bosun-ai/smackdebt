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

Correct workspace dependency resolution SHALL be part of that evidence: release
review SHALL confirm that cross-package references resolved by declared manifest
name appear as internal edges, that ambiguous names keep their diagnostics, and
that no coupling output claims an absent code dependency for an explained pair.

#### Scenario: Workspace resolution is reviewed but later changes remain
- **WHEN** `resolve-workspace-dependencies` passes but any following v-next change is not archived
- **THEN** `prepare-first-release` remains blocked and publication evidence is not finalized

#### Scenario: Later change packages are authored early
- **WHEN** the four following v-next change documents exist before this change is implemented
- **THEN** authoring is accepted while their implementation still waits for the preceding change to be archived

#### Scenario: Implementation is reviewed but not committed
- **WHEN** local results look correct before the related implementation revision exists
- **THEN** clean release evidence is not recorded

#### Scenario: Reviewed implementation commit is clean
- **WHEN** every implementation and expectation change is committed
- **THEN** the release workflow records exact public profiles and aggregate workload outcomes from that clean HEAD
