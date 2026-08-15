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

### Requirement: A repository-relative reference is never an external package
A reference rooted at `crate`, `self`, or `super` SHALL name a module inside
the declaring file's own package and SHALL never be classified as an external
package, including when the root segment carries no further path because a
brace list or a glob follows it. `use crate::{A, B};` SHALL be an internal
crate-root reference rather than an external package named `crate`.

A glob import SHALL name the module it globs, not a child named `*`. A
reference whose remaining path cannot be expressed as a repository path — the
module of an inline module, or an item declared at the crate root — SHALL emit
the matching symbolic candidate for that role in addition to any path
candidates it can express, so path candidates keep precedence. Rust extraction
SHALL NOT read the filesystem to decide any of this.

#### Scenario: A brace list follows the crate root
- **WHEN** a Rust file contains `use crate::{First, Second};`
- **THEN** the reference is internal and names the crate root, and no external package named `crate` is counted

#### Scenario: A glob import names its module
- **WHEN** a Rust file contains `use super::*;` inside an inline module
- **THEN** extraction emits the declaring-file candidate rather than a candidate for a child named `*`

#### Scenario: An inline module reference keeps its path candidates
- **WHEN** a Rust file contains `use super::sibling::work;` inside an inline module
- **THEN** extraction emits the sibling module path candidates first and the declaring-file candidate last

#### Scenario: A crate-rooted path keeps its module candidates
- **WHEN** a Rust file contains `use crate::core::work;`
- **THEN** extraction emits the module path candidates for `core::work` first and the crate-root candidate last
