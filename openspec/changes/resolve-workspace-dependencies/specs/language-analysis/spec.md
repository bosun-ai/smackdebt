## ADDED Requirements

### Requirement: Dependency extraction emits dependency targets only
A language implementation SHALL emit only the referenced target text of a
dependency form. Visibility, modifier, attribute, and re-export keywords SHALL
be treated as properties of the declaring item and SHALL NOT become a dependency
target or a first path segment. Rust `pub`, `pub(crate)`, `pub(super)`,
`pub(in path)`, and equivalent forms SHALL therefore never appear as a
dependency target.

#### Scenario: Rust re-exports a module path
- **WHEN** a Rust file contains `pub use a::b;`
- **THEN** extraction emits the dependency target `a::b` and emits no target `pub`

#### Scenario: A visibility keyword leads a declaration
- **WHEN** a Rust file contains `pub mod child;`
- **THEN** extraction emits one module-ownership relation for `child` and no dependency target derived from the visibility keyword
