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

Release review SHALL confirm that hotspots, stable-dependency findings,
knowledge-concentration findings, size findings, orphan facts, and history
window coverage are present, that rank places hot production debt first, and
that no contributor identity appears in any output.

#### Scenario: Deepened signals are reviewed but later changes remain
- **WHEN** `deepen-debt-signals` passes but any following v-next change is not archived
- **THEN** `prepare-first-release` remains blocked and publication evidence is not finalized

#### Scenario: A signal reaches no reader
- **WHEN** a computed signal has no report table, rank effect, or evidence
- **THEN** release review rejects it as unfinished rather than accepting it as serialized-only data

#### Scenario: Implementation is reviewed but not committed
- **WHEN** local results look correct before the related implementation revision exists
- **THEN** clean release evidence is not recorded

#### Scenario: Reviewed implementation commit is clean
- **WHEN** every implementation and expectation change is committed
- **THEN** the release workflow records exact public profiles and aggregate workload outcomes from that clean HEAD
