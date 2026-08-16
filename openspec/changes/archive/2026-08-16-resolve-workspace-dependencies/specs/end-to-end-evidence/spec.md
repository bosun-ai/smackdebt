## ADDED Requirements

### Requirement: Workspace resolution has generated fixture evidence
Public generated repositories SHALL include a workspace fixture per supported
manifest kind — Cargo with a `[lib] name` override, npm with a scoped name,
Python `pyproject.toml`, and a gemspec — whose cross-package references are
written against declared names rather than paths. Exact acceptance SHALL prove
that those references become internal package `uses` edges, that package degree
and at least one package cycle appear only because of manifest-name resolution,
that a shadowed duplicate manifest name stays ambiguous with its diagnostic,
that a Rust `pub use a::b;` yields no `pub` dependency target, that one package
pair yields exactly one coupling row, that no ancestor-descendant pair exists,
and that no output claims `no code dependency` for a pair connected by a
manifest-name edge. JSON version 3 structure SHALL be proven unchanged while its
values move, and serial and parallel runs SHALL remain byte-identical.

#### Scenario: A workspace fixture is analyzed
- **WHEN** the real CLI analyzes a workspace fixture whose packages import each other by declared name
- **THEN** the result contains internal package edges, exact degree facts, and no external classification for those references

#### Scenario: A duplicate manifest name is present
- **WHEN** two fixture packages declare the same name and a third imports it
- **THEN** the reference is ambiguous with a retained diagnostic and no internal edge is created

#### Scenario: Coupling evidence is audited
- **WHEN** a fixture produces coupling for a package pair and for a scope pair where one contains the other
- **THEN** exactly one row exists for the package pair and no row exists for the containing pair
