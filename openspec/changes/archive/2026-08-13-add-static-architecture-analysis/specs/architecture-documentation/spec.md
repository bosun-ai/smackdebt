## ADDED Requirements

### Requirement: Architecture documents static dependency flow
`ARCHITECTURE.md` SHALL describe language-owned dependency syntax,
project-owned repository indexing and safe resolution, analysis-owned flat
graphs and algorithms, project composition, and borrowed output.

#### Scenario: Engineer changes dependency support
- **WHEN** a new import form or resolution rule is needed
- **THEN** the guide identifies whether syntax translation or repository resolution owns the change

### Requirement: Architecture documents graph meaning and limits
The architecture guide SHALL define internal, external, unresolved, and
ambiguous references; file and package edge aggregation; cycles; witnesses;
fan-in; fan-out; instability; health rules; and unsupported compiler or runtime
resolution.

#### Scenario: User questions a missing edge
- **WHEN** a maintainer traces the implementation contract
- **THEN** the guide explains when Smackdebt refuses to guess and how the coverage gap remains visible

### Requirement: Architecture documents graph performance
The architecture guide SHALL describe single-parse extraction, flat edge
ownership, stable ordering, complete affected diff graphs, generated graph
workloads, and memory growth relative to observed relationships.

#### Scenario: Engineer changes graph construction
- **WHEN** the change affects edge ownership or ordering
- **THEN** the guide identifies the correctness, deterministic-output, allocation, and complete-flow performance evidence to rerun
