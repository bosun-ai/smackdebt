## MODIFIED Requirements

### Requirement: JSON evidence validates structure and meaning
Every acceptance JSON result SHALL validate against the version-4 schema and pass semantic,
index, privacy, and exact-byte checks from the same invocation before its
performance sample is accepted. Validation SHALL prove that the denormalized
head agrees with the tables it duplicates, that hotspots, size findings, orphan
files, finding kind discriminators, history window fields, and `manifest_name`
are present and consistent, and that no floating-point number appears anywhere
in the object.

#### Scenario: A recovered High fact is emitted
- **WHEN** JSON validation runs
- **THEN** the advisory fact remains present and no health, graph-verdict, coupling-finding, or diff-verdict index refers to it

#### Scenario: A private value reaches output
- **WHEN** output contains source, an absolute private path, Git identity, or private history detail
- **THEN** privacy validation fails before evidence approval

#### Scenario: The head disagrees with a table
- **WHEN** a summary count or worst entry differs from the table facts it duplicates
- **THEN** validation fails before exact-byte approval

#### Scenario: A float is serialized
- **WHEN** any value in the object is a floating-point number
- **THEN** validation fails before exact-byte approval
