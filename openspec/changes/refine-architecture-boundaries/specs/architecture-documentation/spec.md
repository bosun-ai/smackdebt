## ADDED Requirements

### Requirement: Architecture documents the crate dependency graph
`ARCHITECTURE.md` SHALL name all seven crates, their responsibilities, their
forbidden responsibilities, and every allowed workspace dependency direction.
It SHALL include a focused Mermaid diagram that matches the enforced Cargo graph.

#### Scenario: Engineer reviews a dependency change
- **WHEN** a workspace dependency is added or moved
- **THEN** the architecture document and dependency check provide one matching rule for whether the edge is allowed

### Requirement: Architecture documents hot-path ownership
`ARCHITECTURE.md` SHALL identify where paths, source buffers, unit facts,
findings, parser state, worker pools, and output buffers are owned and released.

#### Scenario: Engineer changes a hot-path value
- **WHEN** a change adds ownership, cloning, or retained state to analysis
- **THEN** the architecture document identifies the expected lifetime and performance rule to preserve

### Requirement: Architecture documents language migration
`ARCHITECTURE.md` SHALL distinguish verified upstream-backed languages from
owned Ruby and Vue analysis, explain why Kotlin is excluded, and define the
language-local replacement process.

#### Scenario: Engineer adds an owned replacement
- **WHEN** an upstream-backed language is replaced
- **THEN** the architecture document identifies the fixtures, performance evidence, and isolated registry switch required

### Requirement: Architecture documents evidence gates
`ARCHITECTURE.md` SHALL describe structural performance checks, private and
public benchmark workloads, captured environment metadata, and the process for
setting and revising absolute budgets.

#### Scenario: Engineer evaluates a performance change
- **WHEN** a change affects walking, reading, parsing, Git, aggregation, or output
- **THEN** the architecture document identifies the focused measurements and comparison boundary required
