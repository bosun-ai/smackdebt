## ADDED Requirements

### Requirement: Version 4 discloses discovery and coverage honesty
The diagnostics table's kind vocabulary SHALL include `nested_repository` for
a pruned nested git checkout, carrying its repository-relative path like other
diagnostics. Coverage SHALL expose `selected_bytes` and `unsupported_bytes`
as integer byte totals of the selected scope. The `verdict` head SHALL carry
the analysis-owned unsupported-coverage qualifier beside `tier` and
`sentence` when the qualifier exists and SHALL omit it otherwise. The checked
version-4 schema SHALL validate all three additions, and every serialized
value SHALL remain an integer or a string.

#### Scenario: A nested checkout was pruned
- **WHEN** a report over a repository containing a nested checkout is serialized
- **THEN** one diagnostic row carries kind `nested_repository` and the pruned path

#### Scenario: Coverage bytes are serialized
- **WHEN** any version-4 report is emitted
- **THEN** coverage carries integer `selected_bytes` and `unsupported_bytes` that validate against the schema

#### Scenario: An unqualified verdict is serialized
- **WHEN** the unsupported share is at or below the threshold
- **THEN** the verdict head carries no qualifier member rather than an empty one
