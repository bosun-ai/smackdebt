## MODIFIED Requirements

### Requirement: A repository-relative reference is never an external package
A reference rooted at `crate`, `self`, or `super` SHALL name a module inside
the declaring file's own package and SHALL never be classified as an external
package, including when the root segment carries no further path because a
brace list or a glob follows it. Each member of a brace list rooted at
`crate` SHALL be its own internal reference, and no member SHALL be counted
as an external package named `crate`.

A glob import SHALL name the module it globs, not a child named `*`. A
reference whose remaining path cannot be expressed as a repository path — the
module of an inline module, or an item declared at the crate root — SHALL emit
the matching symbolic candidate for that role in addition to any path
candidates it can express, so path candidates keep precedence. Rust extraction
SHALL NOT read the filesystem to decide any of this.

#### Scenario: A brace list follows the crate root
- **WHEN** a Rust file contains `use crate::{First, Second};`
- **THEN** two internal references are emitted, one per member, and no external package named `crate` is counted

#### Scenario: A glob import names its module
- **WHEN** a Rust file contains `use super::*;` inside an inline module
- **THEN** extraction emits the declaring-file candidate rather than a candidate for a child named `*`

#### Scenario: An inline module reference keeps its path candidates
- **WHEN** a Rust file contains `use super::sibling::work;` inside an inline module
- **THEN** extraction emits the sibling module path candidates first and the declaring-file candidate last

#### Scenario: A crate-rooted path keeps its module candidates
- **WHEN** a Rust file contains `use crate::core::work;`
- **THEN** extraction emits the module path candidates for `core::work` first and the crate-root candidate last

## ADDED Requirements

### Requirement: Grouped Rust imports resolve each imported item
A Rust `use` declaration containing a brace list SHALL emit one reference per
imported item, with each item's path reconstructed from its enclosing list
prefixes. Nested lists SHALL resolve recursively; an `as` clause SHALL
reference the original item, not the alias; a `self` member SHALL reference
the enclosing list's own path; a wildcard member SHALL follow the existing
glob rule; and a re-exported list (`pub use`) SHALL resolve identically to a
plain one. Each emitted item SHALL carry its own source span, and a member
SHALL be counted exactly once. Test-scope role demotion SHALL apply to every
emitted item exactly as it applies to a single-target declaration.

#### Scenario: A nested list is imported
- **WHEN** a Rust file contains `use crate::{a::{B, C}, d};`
- **THEN** extraction emits references for `a::B`, `a::C`, and `d` and nothing else

#### Scenario: A list is re-exported
- **WHEN** a Rust file contains `pub use crate::{A, B};`
- **THEN** extraction emits one reference per item exactly as for a plain grouped use

#### Scenario: A list member is aliased
- **WHEN** a Rust file contains `use crate::{long_name as short};`
- **THEN** extraction references `long_name` and emits no target for the alias

#### Scenario: A list contains self
- **WHEN** a Rust file contains `use crate::module::{self, Item};`
- **THEN** extraction references `module` for the `self` member and `module::Item` for the item

#### Scenario: A grouped use is test-scoped
- **WHEN** a grouped use sits under a `#[cfg(test)]` scope
- **THEN** every emitted item carries the test role
