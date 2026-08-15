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
