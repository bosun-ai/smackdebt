# release-readiness Specification

## Purpose
TBD - created by archiving change prove-unified-analysis. Update Purpose after archive.
## Requirements
### Requirement: Unified analysis evidence precedes publication
The workspace SHALL remain private until the exact short terminal interface,
glyph and styling policy, simple human text, unchanged JSON and analysis
behavior, public width matrix, resource gates, installed behavior, and three
privacy-safe workload outcomes pass from the reviewed implementation commit.

#### Scenario: Implementation is reviewed but not committed
- **WHEN** local terminal results look correct before the implementation commit exists
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

