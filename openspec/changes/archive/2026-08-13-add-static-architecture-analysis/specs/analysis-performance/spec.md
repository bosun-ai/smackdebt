## ADDED Requirements

### Requirement: Dependency extraction shares source work
Static dependency extraction SHALL use the same source read, parse, and node
classification as source measurements. It SHALL NOT add another filesystem walk,
source read, or parse per file.

#### Scenario: Supported file has source metrics and imports
- **WHEN** one project worker analyzes the file
- **THEN** one source read and one parse produce both unit measurements and dependency syntax

### Requirement: Graph storage follows observed relationships
File and package graph storage SHALL use flat reserved tables and adjacency
indexes proportional to resolved internal relationships. Graph algorithms SHALL
run without cloning source paths or complete edge tables.

#### Scenario: Large sparse repository is analyzed
- **WHEN** most source files have few internal dependencies
- **THEN** graph memory follows nodes and observed edges rather than every possible file pair

### Requirement: Graph construction and analysis are deterministic
Parallel file analysis SHALL produce stable edge, component, witness, finding,
and comparison order after one explicit stable ordering step. Serial and
parallel terminal and JSON bytes SHALL match.

#### Scenario: Several workers discover cycle edges
- **WHEN** completion order differs between runs
- **THEN** final graph tables and cycle witnesses remain byte-for-byte identical

### Requirement: Static architecture performance uses complete flows
Generated performance checks SHALL cover sparse graphs, dense package graphs,
many packages, and small and large dependency diffs. Correctness and graph
coverage SHALL be checked outside measured intervals before wall time,
allocations, and peak memory are accepted.

#### Scenario: Graph workload changes
- **WHEN** its package or edge shape changes materially
- **THEN** the repository records a new workload identity and reviewed baseline
