# report-schema-v2 Specification

## Purpose
TBD - created by archiving change add-evolutionary-architecture-analysis. Update Purpose after archive.
## Requirements
### Requirement: JSON version 2 includes evolutionary tables

The system SHALL represent history coverage, file history, package history,
change coupling, contributor concentration, and evolutionary findings as flat
indexed version-2 tables.

#### Scenario: JSON contains retained history

- **WHEN** a repository has usable local history
- **THEN** file and package history rows contain touch and textual churn counts
- **AND** coupling rows contain package indexes, shared count, union count, and
  similarity
- **AND** concentration rows contain package index, contributor count,
  numerator, denominator, and ratio

#### Scenario: JSON is inspected for identity data

- **WHEN** version-2 JSON is emitted from fixtures with known author names and
  addresses
- **THEN** the schema contains no author identity table or field
- **AND** the emitted bytes contain none of those fixture identities

### Requirement: JSON records the history window and coverage

The system SHALL identify the analyzed revision, locally available history
window, completeness state, counted textual changes, uncounted changes,
excluded paths, and rename gaps.

#### Scenario: History is incomplete

- **WHEN** a shallow or partially readable repository is analyzed
- **THEN** JSON reports incomplete history coverage and its reason
- **AND** retained measurements are not marked complete

### Requirement: Version-2 examples are executable evidence

The system SHALL keep committed codebase, ref-diff, and worktree-diff JSON
version-2 results that validate against the committed schema and match generated
fixture facts.

#### Scenario: The schema or report changes

- **WHEN** a version-2 field, table, index relationship, or enum changes
- **THEN** schema validation or exact result comparison fails
- **AND** acceptance requires an intentional schema, documentation, and expected
  result update

### Requirement: JSON and terminal consume the same analysis result

The system SHALL prove through test-only work counts that selecting JSON or
terminal output does not rerun discovery, reads, parsing, Git history, or
analysis.

#### Scenario: Both formats render equivalent fixture facts

- **WHEN** terminal and JSON commands analyze the same fixture state
- **THEN** focused assertions find the same code, architecture, evolution,
  comparison, and coverage facts
- **AND** each renderer performs presentation work only

