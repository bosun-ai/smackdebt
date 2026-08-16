## ADDED Requirements

### Requirement: Manifest names resolve cross-package references
Discovery SHALL extract the declared package name while recognizing a package
manifest and SHALL store it on the package record. Supported sources SHALL be
`Cargo.toml` `[package] name` with a `[lib] name` override when present,
`package.json` `name` including a scoped `@scope/name` value, `pyproject.toml`
`[project] name`, and a gemspec's declared name. A missing, empty, or unreadable
declared name SHALL leave the package without a manifest name and SHALL NOT be
an error. Extraction SHALL reuse the existing manifest read and SHALL NOT add a
walk, an extra read, or a process.

Project resolution SHALL consult a read-only manifest-name index only for a
reference that path candidates would otherwise classify external. It SHALL use
the reference's first path segment, normalize hyphens and underscores for Rust,
and compare declared names exactly for other languages. A reference SHALL
resolve to an internal package-level `uses` edge only when exactly one internal
package matches. The edge SHALL target the matched package's entry file when
that file resolves and SHALL otherwise remain package-scoped without a file
target. Resolution SHALL NOT execute or emulate build configuration, lockfiles,
resolver algorithms, workspace inheritance, version constraints, or path
aliases.

#### Scenario: A workspace crate is imported by its declared name
- **WHEN** a Rust file imports `smackdebt_analysis::report` and exactly one internal package declares the manifest name `smackdebt-analysis`
- **THEN** the reference resolves to an internal `uses` edge to that package instead of an external reference

#### Scenario: A scoped npm package is imported
- **WHEN** a JavaScript file imports `@acme/ui/button` and exactly one internal package declares the name `@acme/ui`
- **THEN** the reference resolves to an internal `uses` edge to that package

#### Scenario: A manifest declares no usable name
- **WHEN** a recognized manifest has no readable declared name
- **THEN** the package is still discovered and its references resolve by path candidates only

### Requirement: Internal manifest matches never guess through shadowing
An internal manifest-name match SHALL win only when the normalized name maps to
exactly one internal package. When it maps to more than one internal package,
the reference SHALL remain unresolved or ambiguous, no internal edge SHALL be
created, and its diagnostic and source location SHALL be retained. Path
candidate resolution SHALL keep precedence over manifest-name resolution.

#### Scenario: An internal name is not unique
- **WHEN** two internal packages declare the same manifest name and a reference uses that name
- **THEN** the reference is ambiguous, no internal edge is created, and the ambiguity diagnostic retains its source location

#### Scenario: An external package shares an internal name
- **WHEN** a repository depends on a published package whose name equals a unique internal package name
- **THEN** the internal package wins the match and the retained diagnostics keep the resolution reviewable

#### Scenario: A path candidate already matched
- **WHEN** a reference resolves through an existing path candidate
- **THEN** the manifest-name index is not consulted for that reference

### Requirement: Symbolic candidates resolve a role, not a path
A symbolic candidate SHALL name a file by its role in the declaring file's own
package instead of by a repository path, and a language MAY emit one where no
repository path can express the reference. Two symbolic candidates SHALL exist:
the declaring file itself, and the module root of the declaring file's package.
Project resolution SHALL resolve the declaring-file candidate to the file that
declares the reference, and SHALL resolve the crate-root candidate to that
package's module root file, preferring `lib.rs` over `main.rs` when both exist.

A symbolic candidate SHALL be consulted only when no path candidate of the same
reference matched a discovered file, so path resolution and its
exactly-one-match rule keep precedence and their ambiguity outcomes are
unchanged. A symbolic candidate that resolves to the declaring file SHALL count
as resolved-internal coverage without creating a self edge. A symbolic candidate
that resolves to no discovered file SHALL leave the reference unresolved with
its diagnostic.

#### Scenario: A relative reference names an item of the declaring file
- **WHEN** an inline test module contains `use super::*;` and no sibling module path matches
- **THEN** the reference resolves to the declaring file, counts as resolved-internal, and creates no edge

#### Scenario: A sibling module path still wins
- **WHEN** a reference inside an inline module offers both a path candidate that matches a discovered file and the declaring-file candidate
- **THEN** the path candidate resolves the reference and the declaring-file candidate is not consulted

#### Scenario: A crate-rooted reference names a crate-root item
- **WHEN** a Rust file contains `use crate::Item;` and no module file named `Item` exists
- **THEN** the reference resolves to the crate root file of its own package instead of staying unresolved

#### Scenario: A symbolic candidate matches nothing
- **WHEN** a crate-rooted reference has no discoverable crate root file
- **THEN** the reference stays unresolved and keeps its diagnostic and source location

## MODIFIED Requirements

### Requirement: Static dependency coverage is visible
Every extracted reference SHALL contribute to resolved-internal, external,
unresolved, or ambiguous coverage. Unresolved and ambiguous facts SHALL retain
their source location and reason. External references SHALL not participate in
the internal graph. A reference that resolves internally through a unique
manifest-name match SHALL count as resolved-internal rather than external, and
SHALL NOT be counted twice.

#### Scenario: Repository uses external packages
- **WHEN** a reference safely identifies an external package but no internal file and no unique internal manifest name
- **THEN** the package receives an external dependency count without an internal graph edge

#### Scenario: Some references cannot be resolved
- **WHEN** a codebase report otherwise succeeds
- **THEN** architecture output states exact unresolved and ambiguous counts rather than treating coverage as complete

#### Scenario: A workspace import stops being external
- **WHEN** manifest-name resolution matches a reference previously counted as external
- **THEN** external coverage decreases by that reference and resolved-internal coverage increases by the same reference
