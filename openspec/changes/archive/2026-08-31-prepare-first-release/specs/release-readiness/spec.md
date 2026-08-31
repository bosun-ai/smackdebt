## ADDED Requirements

### Requirement: Publication requires separate evidence

The workspace SHALL remain private until installation, package contents,
licenses, provenance, command behavior, JSON behavior, and release workloads
pass from a clean revision.

#### Scenario: Development implementation completes

- **WHEN** the private workspace implementation passes
- **THEN** no crate becomes publishable through that implementation change

### Requirement: Publication follows dependency order

Libraries SHALL be published from inward dependencies outward, and the CLI
SHALL be published last.

#### Scenario: Release operator publishes the first version

- **WHEN** all release evidence passes
- **THEN** each library is available before its consumer and the CLI is last
