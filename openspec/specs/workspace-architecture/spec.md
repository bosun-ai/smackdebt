# workspace-architecture Specification

## Purpose
TBD - created by archiving change refine-architecture-boundaries. Update Purpose after archive.
## Requirements
### Requirement: Workspace uses responsibility-led crates
The implementation SHALL use separate crates for pure analysis, language
analysis, filesystem discovery, Git access, project orchestration, output, and
the CLI. Each crate SHALL own one documented responsibility, SHALL organize its
behavior in private responsibility-led modules, and SHALL export only items
required by direct consumers.

#### Scenario: Engineer places new behavior
- **WHEN** an engineer adds parser, Git, policy, scheduling, rendering, or CLI behavior
- **THEN** exactly one documented crate and one focused module own that behavior
  and unrelated crates do not gain the responsibility

### Requirement: Dependencies point toward pure analysis
`smackdebt-analysis` SHALL have no filesystem, Git, parser, Rayon, Serde, or
terminal dependency. Language and discovery SHALL depend on analysis for shared
domain facts. Git SHALL have no Smackdebt dependency. Output SHALL depend only
on analysis among Smackdebt crates. Project SHALL compose analysis, language,
discovery, and Git, while the CLI SHALL depend only on project and output.

#### Scenario: Workspace dependency graph is checked
- **WHEN** CI reads Cargo metadata for non-development workspace dependencies
- **THEN** every edge matches the documented dependency direction or the check fails

### Requirement: Public Rust surfaces stay minimal
Workspace libraries SHALL export only behavior and values required by direct
crate consumers. Report assembly, parser types, upstream metrics, syntax nodes,
Rayon types, Git process handles, callback systems, status parsing, and output
helpers SHALL remain private.

#### Scenario: Public API changes
- **WHEN** a workspace library's compiler-visible Rust surface changes
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
Discovery SHALL assign the analysis-owned package identity. A completed report
SHALL own each repository-relative path once and SHALL refer to shared package,
path, scope, file, finding, diagnostic, and comparison facts through typed
indexes.

#### Scenario: One finding appears in several scope summaries
- **WHEN** a debt finding contributes to file, directory, package, and repository views
- **THEN** the finding and its path are stored once and each scope refers to the retained fact

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
`ProjectReport` SHALL own the initial selected scope. `Report` SHALL contain
completed immutable facts without mutable navigation or construction state,
and renderers SHALL receive the selected scope explicitly.

#### Scenario: Another renderer is added
- **WHEN** a terminal interface or editor view consumes a completed report
- **THEN** it can select retained scopes without adding filesystem, Git, parser,
  Rayon, serialization, or navigation state to analysis policy

### Requirement: Comparisons identify their file once
Every diff comparison SHALL refer to one file through a typed index. Scope
summaries SHALL refer to retained comparisons by index rather than copying
comparison values.

#### Scenario: One regression appears in ancestor summaries
- **WHEN** a comparison contributes to file, directory, package, and repository counts
- **THEN** the comparison is stored once and each applicable scope holds its index

### Requirement: Codebase and diff share hierarchy construction
Project report construction SHALL use one private hierarchy policy for
repository, package, directory, and file scopes in both report modes. It SHALL
pass completed scopes and facts through the analysis-owned report builder
rather than mutating a completed report.

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

### Requirement: Terminal presentation separates selection from layout
The output crate SHALL build private borrowed presentation rows from one report
before choosing a width layout. Ranking, omission, and navigation decisions
SHALL occur once, and renderers SHALL not run analysis or inspect filesystem,
Git, environment, or terminal state.

#### Scenario: Several terminal widths render one report
- **WHEN** full, compact, and stacked layouts render the same completed report
- **THEN** they consume the same selected presentation rows and differ only in layout

#### Scenario: A later renderer needs progressive rows
- **WHEN** another in-process terminal renderer is introduced
- **THEN** the private presentation boundary can be reused without moving terminal state into the report domain

### Requirement: Report assembly belongs to analysis
Analysis SHALL expose one small report-construction interface that inserts
indexed facts, links retained facts, interns paths, and aggregates a completed
report. Project SHALL keep use-case-specific hierarchy and result assembly
private.

#### Scenario: Project completes analysis work
- **WHEN** project orchestration has a source result for a selected file
- **THEN** it passes completed facts to report construction without mutating a
  completed report, aggregating tables itself, or cloning complete report tables

### Requirement: Git owns worktree change merging
The Git crate SHALL combine base changes with worktree state and return sorted,
unique changes with current and base source identity.

#### Scenario: One path appears in staged and unstaged state
- **WHEN** project requests changes from a ref
- **THEN** Git returns that path once with enough information to read both sides

