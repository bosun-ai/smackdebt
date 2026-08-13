## ADDED Requirements

### Requirement: Unified analysis evidence precedes publication

The workspace SHALL remain private until the accepted source-engine, static
architecture, evolutionary-analysis, and unified-evidence changes pass their
complete validation from a clean revision.

#### Scenario: Implementation is complete but evidence is partial

- **WHEN** any required generated fixture, public flow, schema check, exact
  result, installed smoke test, or complete performance record is missing
- **THEN** no crate becomes publishable

#### Scenario: Clean-revision evidence passes

- **WHEN** every required lower-level test and black-box matrix case passes from
  a clean revision
- **THEN** the recorded release evidence identifies the workload, revision,
  toolchain, host, correctness results, resources, reads, and Git processes
- **AND** publication still requires its separate explicit release action

### Requirement: Release evidence includes installed behavior

The release gate SHALL run help, version, terminal, and JSON behavior through a
temporarily installed command outside the source workspace.

#### Scenario: Workspace-only assumptions remain

- **WHEN** the installed command cannot analyze the generated external
  repository with locked dependencies
- **THEN** release evidence fails even if workspace tests pass

