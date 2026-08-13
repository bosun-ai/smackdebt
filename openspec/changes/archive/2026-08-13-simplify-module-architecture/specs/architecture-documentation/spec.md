## ADDED Requirements

### Requirement: Architecture documents module placement
`ARCHITECTURE.md` SHALL describe entry-module restrictions, private module
responsibilities, visibility rules, and the interfaces between the seven crates.

#### Scenario: Engineer extracts or adds behavior
- **WHEN** an engineer chooses where a type or function belongs
- **THEN** the architecture guide identifies both the owning crate and the
  focused private module rule

### Requirement: Architecture documents report construction ownership
`ARCHITECTURE.md` SHALL describe the analysis-owned report builder, immutable
completed report, project-owned selected scope, shared path ownership, and Git
change interface.

#### Scenario: Engineer changes report assembly
- **WHEN** report construction or selection behavior changes
- **THEN** the architecture guide identifies the owner and prevents project or
  output from mutating report tables
