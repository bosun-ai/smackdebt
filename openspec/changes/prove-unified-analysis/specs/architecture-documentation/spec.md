## ADDED Requirements

### Requirement: Architecture documentation defines evidence layers

`ARCHITECTURE.md` SHALL distinguish pure policy tests, language truth fixtures,
adapter tests, black-box CLI evidence, allocation checks, and performance
workloads and SHALL state what each layer can prove.

#### Scenario: A maintainer changes an algorithm

- **WHEN** the maintainer reads the test architecture
- **THEN** the document directs exact policy changes to pure truth tests
- **AND** directs public behavior changes to schema and black-box evidence

### Requirement: Architecture documentation defines the acceptance seam

`ARCHITECTURE.md` SHALL state that acceptance invokes the built process and may
observe test-only work counts but does not construct internal reports or repeat
analysis policy.

#### Scenario: A maintainer extends the harness

- **WHEN** a new public flow is added
- **THEN** the documented seam requires process status and stream assertions
- **AND** prevents the harness from bypassing CLI composition

