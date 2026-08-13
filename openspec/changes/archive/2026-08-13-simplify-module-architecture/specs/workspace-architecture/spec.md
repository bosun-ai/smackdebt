## MODIFIED Requirements

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

### Requirement: Core facts use one ownership point
Discovery SHALL assign the analysis-owned package identity. A completed report
SHALL own each repository-relative path once and SHALL refer to shared package,
path, scope, file, finding, diagnostic, and comparison facts through typed
indexes.

#### Scenario: One finding appears in several scope summaries
- **WHEN** a debt finding contributes to file, directory, package, and repository views
- **THEN** the finding and its path are stored once and each scope refers to the retained fact

### Requirement: Scope selection is separate from report facts
`ProjectReport` SHALL own the initial selected scope. `Report` SHALL contain
completed immutable facts without mutable navigation or construction state,
and renderers SHALL receive the selected scope explicitly.

#### Scenario: Another renderer is added
- **WHEN** a terminal interface or editor view consumes a completed report
- **THEN** it can select retained scopes without adding filesystem, Git, parser,
  Rayon, serialization, or navigation state to analysis policy

### Requirement: Codebase and diff share hierarchy construction
Project report construction SHALL use one private hierarchy policy for
repository, package, directory, and file scopes in both report modes. It SHALL
pass completed scopes and facts through the analysis-owned report builder
rather than mutating a completed report.

#### Scenario: A changed file belongs to a nested package
- **WHEN** codebase and diff reports cover the same repository-relative file
- **THEN** both assign it through the same package, directory, and file path structure

## ADDED Requirements

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
