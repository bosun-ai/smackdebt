# release-readiness Specification

## Purpose
TBD - created by archiving change prove-unified-analysis. Update Purpose after archive.
## Requirements
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
remain blocked until all five are archived and SHALL be permitted to resume
after the fifth is archived.

Version 4 SHALL be the only machine contract at publication, which satisfies the
earlier requirement to retire version 3 before the first release. Release review
SHALL confirm that no version-2 or version-3 object, schema, or documentation
statement is presented as supported.

#### Scenario: All five v-next changes are archived
- **WHEN** the fifth change is archived with reviewed proof
- **THEN** `prepare-first-release` is permitted to resume its own evidence and publication tasks

#### Scenario: An older schema version is still presented
- **WHEN** any documentation, example, or CLI behavior presents version 2 or version 3 as supported
- **THEN** release evidence fails

#### Scenario: Implementation is reviewed but not committed
- **WHEN** local results look correct before the related implementation revision exists
- **THEN** clean release evidence is not recorded

#### Scenario: Reviewed implementation commit is clean
- **WHEN** every implementation and expectation change is committed
- **THEN** the release workflow records exact public profiles and aggregate workload outcomes from that clean HEAD

### Requirement: Release evidence includes installed behavior

The release gate SHALL run help, version, terminal, and JSON behavior through a
temporarily installed command outside the source workspace.

#### Scenario: Workspace-only assumptions remain

- **WHEN** the installed command cannot analyze the generated external
  repository with locked dependencies
- **THEN** release evidence fails even if workspace tests pass

### Requirement: One post-commit workflow records release evidence
The release workflow SHALL require a clean tree after the reviewed
implementation commit, capture one HEAD, toolchain, and host state, and record
all public profiles plus privacy-safe aggregate results for self, private mixed
application, and private Rust workspace. Every record SHALL use the captured
revision even after evidence files are written.

#### Scenario: Complete recording succeeds
- **WHEN** one documented command runs from clean reviewed HEAD
- **THEN** the checker requires `workspace_dirty` false and every revision equal to that HEAD

#### Scenario: Work exists before recording
- **WHEN** the tree differs from reviewed HEAD
- **THEN** recording stops without replacing accepted evidence
