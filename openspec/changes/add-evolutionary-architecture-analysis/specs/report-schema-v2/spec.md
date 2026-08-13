## ADDED Requirements

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

