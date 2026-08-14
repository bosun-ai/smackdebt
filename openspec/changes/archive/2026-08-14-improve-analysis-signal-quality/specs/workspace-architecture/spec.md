## ADDED Requirements

### Requirement: Package table identity is stable

Discovery SHALL create one package row per recognized package root in stable
repository-relative path order before source selection. Each row SHALL receive
one typed package ID that codebase, path, and diff report facts reuse without
compaction or renumbering.

#### Scenario: A manifest package contains no selected source

- **WHEN** path selection excludes every source file in one recognized package
- **THEN** its package row and ID remain present with zero selected-source summaries
- **AND** later package IDs do not change

#### Scenario: The same repository is drilled by path

- **WHEN** repository and package views retain a fact for the same file
- **THEN** that file refers to the same package ID in both reports

### Requirement: Diff package identity states side presence

Current package roots SHALL retain their current package IDs. A package found
only in the comparison base SHALL be appended in stable base-path order and
SHALL state that it is base-only. Removed or empty packages SHALL NOT cause
another package to inherit their ID.

#### Scenario: A package is removed on the current side

- **WHEN** a ref diff contains source from a base-only package
- **THEN** the package table retains one base-only row and comparisons refer to its stable ID

### Requirement: Package paths have one owner

The versioned report SHALL store each package path once on the package table.
Files, package graph measurements, history, coupling, findings, comparisons,
scopes, and output SHALL refer to package IDs instead of copying package paths
or reconstructing identity from scope position.

#### Scenario: One package participates in several analysis families

- **WHEN** a package has code findings, graph edges, churn, and coupling
- **THEN** each fact reaches the package path through the same package-table row
