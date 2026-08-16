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

Release review SHALL confirm that every published report states one tier id from
the frozen vocabulary, that its sentence comes from analysis, and that the diff
verdict reconciles source, architecture, and evolutionary comparisons.

#### Scenario: Verdict policy is reviewed but later changes remain
- **WHEN** `add-verdict-policy` passes but any following v-next change is not archived
- **THEN** `prepare-first-release` remains blocked and publication evidence is not finalized

#### Scenario: A tier id changes
- **WHEN** a proposed change renames or removes a frozen tier id
- **THEN** release review treats it as a machine-contract break requiring its own review

#### Scenario: Implementation is reviewed but not committed
- **WHEN** local results look correct before the related implementation revision exists
- **THEN** clean release evidence is not recorded

#### Scenario: Reviewed implementation commit is clean
- **WHEN** every implementation and expectation change is committed
- **THEN** the release workflow records exact public profiles and aggregate workload outcomes from that clean HEAD
