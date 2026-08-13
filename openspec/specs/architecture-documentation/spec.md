# architecture-documentation Specification

## Purpose
TBD - created by archiving change add-product-foundation. Update Purpose after archive.
## Requirements
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

### Requirement: Architecture defines progressive report navigation
The architecture document SHALL describe report path ownership, selected-scope
ownership, codebase and diff hierarchy construction, finding and comparison
links, and the separation between exact aggregation and display policy.

#### Scenario: Engineer adds another renderer
- **WHEN** a renderer needs repository, package, directory, file, or code-unit detail
- **THEN** the architecture identifies how it selects and traverses retained facts without running analysis

### Requirement: Architecture defines distribution arithmetic
The architecture document SHALL define codebase debt share, diff change share,
three-way diff direction, severity-led ordering, zero denominators, integer
rounding, and smart single-child display behavior.

#### Scenario: Terminal and another renderer show one scope
- **WHEN** both renderers display distribution for the same selected scope
- **THEN** they derive counts and shares from the same documented report facts

### Requirement: Architecture defines selected-path work limits
The architecture document SHALL explain how an explicit path retains
repository-relative identity while limiting discovery, source reads, and share
denominators to the selected area.

#### Scenario: Engineer changes path selection
- **WHEN** selection behavior is updated
- **THEN** the architecture identifies the identity, package-ancestor, source-read, and performance rules that must remain true

### Requirement: Architecture documents package identity flow
Architecture documentation SHALL state that discovery owns codebase package
assignment and hierarchy construction consumes that identity.

#### Scenario: Maintainer changes hierarchy construction
- **WHEN** the maintainer reads the architecture guide
- **THEN** the guide makes the single package-assignment owner explicit

