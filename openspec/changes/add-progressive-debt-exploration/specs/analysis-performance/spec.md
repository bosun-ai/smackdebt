## ADDED Requirements

### Requirement: Explicit paths limit codebase work
Inside a Git repository, an explicit codebase path SHALL preserve
repository-relative identity while discovery and source reads stay limited to
the selected directory subtree or selected file. Package ancestor inspection
SHALL visit each distinct ancestor directory at most once.

#### Scenario: User inspects one nested directory
- **WHEN** the repository contains source outside the selected directory
- **THEN** instrumentation observes no source reads outside the selection and displayed paths retain their repository-relative prefix

### Requirement: Diff package discovery avoids repeated ancestor work
Diff hierarchy construction SHALL inspect each distinct changed-path ancestor at
most once and SHALL retain changed manifest paths before filtering non-source
changes. Git and filesystem work SHALL NOT grow by one process or repeated
ancestor scan per changed source file.

#### Scenario: Many changed files share one package
- **WHEN** a diff contains many files below the same directory chain
- **THEN** package discovery inspects each directory in that chain once and Git process count remains within the existing fixed shape

### Requirement: Scope rendering performs no project work
Rendering a retained scope SHALL perform no inventory walk, source read, Git
operation, parsing, health assessment, or worker scheduling.

#### Scenario: Root report renders several paths
- **WHEN** an instrumented test renders repository, directory, and file scopes from one report
- **THEN** all project-work counters remain unchanged after report construction

### Requirement: Progressive links preserve measured report limits
Finding links, comparison links, path indexes, and diff counts SHALL use reserved
flat storage. The generated large-repository workloads SHALL measure their effect
on allocation count and peak memory before release limits are accepted.

#### Scenario: Large hierarchy is aggregated
- **WHEN** scopes reserve the required child-derived link capacity
- **THEN** the post-order aggregation update performs no new heap allocation and every retained item is linked once per applicable scope
