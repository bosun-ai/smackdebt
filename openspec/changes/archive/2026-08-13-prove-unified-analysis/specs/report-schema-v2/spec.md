## ADDED Requirements

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

