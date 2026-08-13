## MODIFIED Requirements

### Requirement: Owned analyzers reuse worker state
Each project worker SHALL reuse language parsers, immutable compiled queries,
query cursors, traversal stacks, and scratch capacity across consecutive files.
Parser and mutable query state SHALL NOT be shared through a mutex across
workers. Each source region SHALL be parsed at most once during one analysis.

#### Scenario: Worker analyzes consecutive files
- **WHEN** one worker receives several files across supported languages
- **THEN** it reuses its per-language parser and scratch capacity while keeping each file result isolated

#### Scenario: Vue document contains an injected script
- **WHEN** document analysis delegates one included script range
- **THEN** the document and included region are each parsed once without copying the complete source file

## ADDED Requirements

### Requirement: Shared source algorithms use one node traversal
The retained source measurements SHALL consume one classification of each node
visited in a rated unit. Adding a measurement SHALL NOT require reparsing the
source or building another owned syntax tree.

#### Scenario: All retained measurements run
- **WHEN** a supported unit is analyzed
- **THEN** one traversal feeds independent cognitive, cyclomatic, and logical-line state

### Requirement: Source engine replacement is measured end to end
Removing the upstream engine SHALL be evaluated with complete correctness-checked
CLI workloads that record parser time, wall time, allocations, peak memory,
supported files, and source bytes. New absolute limits SHALL replace old limits
only after the workload and metric results are declared as a new baseline.

#### Scenario: New engine passes exact fixtures
- **WHEN** every supported language is correct under the new metric definitions
- **THEN** generated codebase and diff workloads establish reviewed performance limits before release
