## ADDED Requirements

### Requirement: Version 2 exposes static architecture tables
JSON schema version 2 SHALL include flat indexed tables for internal file edges,
package edges, external dependency summaries, resolution diagnostics, package
graph measurements, architecture findings, and architecture comparisons.

#### Scenario: Integration reads a codebase architecture
- **WHEN** an internal package edge contributes to a cycle
- **THEN** the integration can traverse from the finding to its packages, witness edges, files, paths, and source locations through indexes

#### Scenario: Integration reads architecture coverage
- **WHEN** references are external, unresolved, or ambiguous
- **THEN** version 2 exposes exact category counts and retained diagnostic reasons

### Requirement: Version 2 keeps code and architecture results distinct
Code findings and comparisons SHALL remain separate from architecture findings
and comparisons while sharing path, scope, file, package, and direction values
where meanings match.

#### Scenario: One package has both finding kinds
- **WHEN** a package contains a High code unit and participates in a High cycle
- **THEN** JSON reports two independently explainable facts rather than one combined rating
