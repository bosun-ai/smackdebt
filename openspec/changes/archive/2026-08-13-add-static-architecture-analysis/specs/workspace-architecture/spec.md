## MODIFIED Requirements

### Requirement: Workspace uses responsibility-led crates
The implementation SHALL use separate crates for pure analysis, language
analysis, filesystem discovery, Git access, project orchestration, output, and
the CLI. Language analysis SHALL own dependency syntax, project orchestration
SHALL own repository indexing and resolution, pure analysis SHALL own graph
values and algorithms, and output SHALL render borrowed graph facts. Each crate
SHALL export only items required by direct consumers.

#### Scenario: Static architecture behavior is added
- **WHEN** an engineer changes grammar syntax, path resolution, graph policy, scheduling, or rendering
- **THEN** the documented crate owner changes without leaking the responsibility into another domain

### Requirement: Dependencies point toward pure analysis
`smackdebt-analysis` SHALL have no filesystem, Git, parser, Rayon, Serde, or
terminal dependency. Language and discovery SHALL depend on analysis for shared
facts. Git SHALL have no Smackdebt dependency. Output SHALL depend only on
analysis among Smackdebt crates. Project SHALL compose analysis, language,
discovery, and Git and SHALL be the only owner of dependency resolution I/O.
The CLI SHALL depend only on project and output.

#### Scenario: Graph dependency is added
- **WHEN** CI reads Cargo metadata after static architecture implementation
- **THEN** graph algorithms remain in analysis and resolution I/O remains in project without a new cross-adapter edge

### Requirement: Core graph facts use one ownership point
Analysis SHALL own each file dependency edge, package dependency edge,
architecture finding, and architecture comparison once in flat tables. Scope
summaries and output SHALL refer to those facts through typed indexes.

#### Scenario: Cycle appears in several scope summaries
- **WHEN** one cycle affects repository, package, and file views
- **THEN** the cycle finding and its witness edges are stored once and each applicable scope holds its index
