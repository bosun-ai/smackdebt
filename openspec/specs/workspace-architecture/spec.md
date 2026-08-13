# workspace-architecture Specification

## Purpose
TBD - created by archiving change refine-architecture-boundaries. Update Purpose after archive.
## Requirements
### Requirement: Workspace uses responsibility-led crates
The implementation SHALL use separate crates for pure analysis, language
analysis, filesystem discovery, Git access, project orchestration, output, and
the CLI. Each crate SHALL own one documented responsibility and SHALL keep its
implementation modules private unless a direct consumer needs an item.

#### Scenario: Engineer places new behavior
- **WHEN** an engineer adds parser, Git, policy, scheduling, rendering, or CLI behavior
- **THEN** exactly one documented crate owns that behavior and unrelated crates do not gain the responsibility

### Requirement: Dependencies point toward pure analysis
`smackdebt-analysis` SHALL have no filesystem, Git, parser, Rayon, Serde, or
terminal dependency. Language, discovery, Git, and output crates SHALL depend
only on analysis among Smackdebt crates. The project crate SHALL compose those
crates, and the CLI SHALL depend only on project and output.

#### Scenario: Workspace dependency graph is checked
- **WHEN** CI reads Cargo metadata for non-development workspace dependencies
- **THEN** every edge matches the documented dependency direction or the check fails

### Requirement: Public Rust surfaces stay minimal
Workspace libraries SHALL export only values required by direct crate consumers.
Parser types, upstream metrics, syntax nodes, Rayon types, Git process handles,
callback systems, and output helpers SHALL remain private.

#### Scenario: Public API changes
- **WHEN** a workspace library's exported Rust surface changes
- **THEN** its checked API snapshot changes and requires intentional review

### Requirement: Product compatibility is limited to CLI and JSON
Before a separate release specification promotes a Rust API, Smackdebt SHALL
promise compatibility only for documented command behavior, exit codes, standard
stream behavior, and versioned JSON fields.

#### Scenario: Internal crate interface changes before release
- **WHEN** an internal Rust interface changes without changing CLI behavior or JSON schema
- **THEN** the change may proceed after workspace consumers and API snapshots are updated

### Requirement: Crates remain private during product development
Every workspace crate SHALL use `publish = false` until a separate release
change authorizes registry publication. Development installation SHALL use the
CLI crate's local path.

#### Scenario: Developer installs an unreleased build
- **WHEN** a developer installs Smackdebt before publication is authorized
- **THEN** the documented command installs the CLI from its workspace path without publishing any crate

### Requirement: Core facts use one ownership point
Inventory SHALL own repository-relative paths and package identities. Analysis
and report values SHALL refer to them through typed indexes where shared
identity is required, and SHALL not copy a path for every aggregate scope.

#### Scenario: One finding appears in several scope summaries
- **WHEN** a debt finding contributes to file, directory, package, and repository views
- **THEN** the finding is stored once and each scope refers to that retained finding

### Requirement: Report storage is flat and progressively addressable
The report SHALL store scopes, findings, diagnostics, and comparisons in flat
collections with stable typed indexes. Scope relationships SHALL support
repository, package, directory, and file traversal without recursive ownership.

#### Scenario: Renderer shows the next scope level
- **WHEN** a renderer receives a report and a selected scope
- **THEN** it can read that scope's direct children and retained findings without running analysis or copying the report

### Requirement: Package identity is rooted in directories
A directory containing one or more recognized manifests SHALL form one report
package. Co-located manifests SHALL describe the same package, and each source
file SHALL belong once to its nearest package-root ancestor.

#### Scenario: Cargo and npm manifests share a directory
- **WHEN** a directory contains both `Cargo.toml` and `package.json`
- **THEN** the report contains one package at that directory and counts each descendant source file once

#### Scenario: Repository has no recognized manifest
- **WHEN** discovery finds no recognized manifest
- **THEN** the repository root is used as one package

### Requirement: Workspace Rust code forbids unsafe code
All Smackdebt workspace crates SHALL forbid unsafe Rust. Low-level code inside
reviewed external parser dependencies is outside this workspace rule.

#### Scenario: Unsafe block enters a workspace crate
- **WHEN** a workspace crate contains an unsafe block or unsafe declaration
- **THEN** compilation fails under workspace lint settings

### Requirement: Report paths have one owner
The completed report SHALL own each repository-relative path once in a flat path
table. Scopes, files, findings through files, comparisons through files, and
renderers SHALL use typed indexes to share those identities.

#### Scenario: One file contributes to several views
- **WHEN** a file contributes to file, directory, package, and repository output
- **THEN** the report stores its path once and those facts reach it through indexes

### Requirement: Scope selection is separate from report facts
`ProjectReport` SHALL identify an initial selected scope without storing mutable
navigation or terminal state in `Report`. Renderers SHALL accept an explicit
scope index.

#### Scenario: Another renderer is added
- **WHEN** a terminal interface or editor view consumes a completed root report
- **THEN** it can select retained scopes without adding filesystem, Git, parser, Rayon, or serialization concerns to analysis policy

### Requirement: Comparisons identify their file once
Every diff comparison SHALL refer to one file through a typed index. Scope
summaries SHALL refer to retained comparisons by index rather than copying
comparison values.

#### Scenario: One regression appears in ancestor summaries
- **WHEN** a comparison contributes to file, directory, package, and repository counts
- **THEN** the comparison is stored once and each applicable scope holds its index

### Requirement: Codebase and diff share hierarchy construction
Project orchestration SHALL use one internal hierarchy construction policy for
repository, package, directory, and file scopes in both report modes.

#### Scenario: A changed file belongs to a nested package
- **WHEN** codebase and diff reports cover the same repository-relative file
- **THEN** both assign it through the same package, directory, and file path structure

### Requirement: Codebase hierarchy uses discovery package assignment
Project orchestration SHALL pass each selected file's discovery-owned package
identity into hierarchy construction. Hierarchy construction SHALL NOT infer a
second package assignment from path prefixes.

#### Scenario: Source exists outside manifest roots
- **WHEN** discovery assigns a source file to its fallback package because no manifest root is its ancestor
- **THEN** hierarchy construction uses that package and completes without a panic

