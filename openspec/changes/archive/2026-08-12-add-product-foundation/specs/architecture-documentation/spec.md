## ADDED Requirements

### Requirement: Architecture separates policy from adapters
The project SHALL provide a root `ARCHITECTURE.md` that keeps source parsing, filesystem discovery, Git access, terminal output, and JSON serialization outside the health and report domain.

#### Scenario: Engineer adds implementation code
- **WHEN** an engineer chooses a module for new behavior
- **THEN** the architecture document identifies the responsible component and its inward-facing interface

### Requirement: Architecture defines the shared report domain
The architecture document SHALL define scopes, source facts, health, activity, comparisons, diagnostics, and the versioned report consumed by terminal and JSON renderers.

#### Scenario: Engineer adds a renderer
- **WHEN** an engineer implements another output format
- **THEN** the renderer can consume the documented report without running source or Git analysis

### Requirement: Architecture defines safe aggregation
The architecture document SHALL explain how nested source metrics become exclusive code-unit facts and how package, directory, and file summaries avoid duplicate counts.

#### Scenario: Engineer aggregates nested functions
- **WHEN** an analyzed function contains another code unit
- **THEN** the architecture directs the engineer to subtract direct child additive totals before rating the parent

### Requirement: Architecture defines Git behavior
The architecture document SHALL define recent-history collection, default-ref discovery, merge-base comparison, worktree inclusion, rename handling, symbol matching, and file-level fallback.

#### Scenario: Engineer implements diff analysis
- **WHEN** a worktree contains committed, staged, unstaged, renamed, or untracked source changes
- **THEN** the architecture states which source versions to compare and when to report a file-level diagnostic

### Requirement: Architecture records performance and privacy rules
The architecture document SHALL require one filesystem walk, one streamed history pass, batched base-object reads, limited source reads, stable output, local processing, and argument-safe Git execution.

#### Scenario: Engineer changes repository analysis
- **WHEN** the change touches filesystem or Git iteration
- **THEN** tests can verify that the implementation does not introduce one Git process per file or upload repository data
