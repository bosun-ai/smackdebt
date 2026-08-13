## ADDED Requirements

### Requirement: Architecture documents generic source analysis
`ARCHITECTURE.md` SHALL describe the private generic language trait, concrete
compiled dispatch, language-owned grammar knowledge, semantic values, one-pass
unit traversal, independent algorithm modules, nested-unit exclusion, and the
analysis-owned output seam.

#### Scenario: Engineer changes a source measurement
- **WHEN** the engineer reads the architecture guide
- **THEN** it identifies whether the change belongs to shared metric policy or one language's syntax translation

### Requirement: Architecture documents parser ownership
`ARCHITECTURE.md` SHALL describe worker-local parsers, queries, cursors, scratch
storage, source and tree lifetimes, Vue included ranges, and the one-parse rule.

#### Scenario: Engineer changes language performance
- **WHEN** parser or query state ownership changes
- **THEN** the guide identifies its worker lifetime and the complete performance evidence to preserve

### Requirement: Architecture documents schema version 2
`ARCHITECTURE.md` SHALL describe version-2 source tables, indexes, retained
detail, aggregate healthy counts, and the absence of a version-1 compatibility
serializer before first release.

#### Scenario: Report fact changes
- **WHEN** an engineer changes a version-2 field or relationship
- **THEN** the guide identifies the owning domain value, checked JSON Schema, and black-box snapshots that must change together
