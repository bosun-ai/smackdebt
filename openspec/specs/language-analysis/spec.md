# language-analysis Specification

## Purpose
TBD - created by archiving change refine-architecture-boundaries. Update Purpose after archive.
## Requirements
### Requirement: Language dispatch is compiled and private
Smackdebt SHALL use one private language identifier and one compile-time dispatch
from a source file to a concrete analyzer. It SHALL NOT load analyzer plugins,
dynamic libraries, or user-provided code.

#### Scenario: Supported file is analyzed
- **WHEN** discovery supplies a path with a verified supported language
- **THEN** the registry selects one compiled analyzer without exposing parser or plugin interfaces

### Requirement: Every analyzer returns Smackdebt facts
Every analyzer SHALL return the same Smackdebt file-analysis values containing
language, line coverage, parse status, and a flat unit list. No upstream syntax
node or metric type SHALL cross the language crate boundary.

#### Scenario: Project consumes different language engines
- **WHEN** one file uses the temporary upstream adapter and another uses an owned analyzer
- **THEN** project orchestration receives the same Smackdebt file-analysis type from both

### Requirement: Units expose only rated measurements
Each unit SHALL expose its name, container identity, kind, original source span, cognitive
complexity, cyclomatic complexity, exclusive logical lines, maximum nesting
depth, and parameter count. All five measurements SHALL be rated measurements
from this change onward, so a unit's rating SHALL be explainable entirely from
the five serialized measurements. Smackdebt SHALL NOT carry upstream Halstead or
maintainability structures into analysis policy or reports.

#### Scenario: Health policy rates a unit
- **WHEN** a language implementation produces a unit
- **THEN** health policy can explain its rating entirely from the five rated measurements

#### Scenario: A unit is rated for nesting alone
- **WHEN** a unit's cognitive, cyclomatic, and statement values are healthy and its maximum nesting depth is 7
- **THEN** its rating is High and the serialized measurements explain why

### Requirement: Upstream support is pinned and verified
The temporary adapter SHALL pin `rust-code-analysis` revision
`37e5d83c056c8cbf827223d5814a93c5218df1a9`. Initial upstream-backed support
SHALL include C/C++, Java, JavaScript/JSX, Python, Rust, and TypeScript/TSX only
after nested-unit fixtures verify meaningful required measurements.

#### Scenario: Verified upstream language is listed
- **WHEN** product documentation lists an upstream-backed language as supported
- **THEN** its fixtures prove non-empty cognitive, cyclomatic, and line behavior for representative control flow

#### Scenario: Kotlin file is discovered
- **WHEN** a Kotlin file is encountered while the pinned engine still has empty required implementations
- **THEN** the file is reported as unsupported coverage rather than analyzed as healthy

### Requirement: Smackdebt bypasses the upstream directory runner
The temporary adapter SHALL call the pinned library for one already-selected
file at a time. It SHALL NOT use the upstream filesystem walker, worker threads,
channels, callbacks, terminal output, or file-output machinery.

#### Scenario: Project analyzes an upstream-backed file
- **WHEN** a project worker supplies an owned source buffer
- **THEN** the adapter moves it into one per-file upstream call and immediately reduces the result to Smackdebt facts

### Requirement: Metric extraction follows metric meaning
The adapter SHALL read direct cognitive and cyclomatic values for each rated
unit. It SHALL derive exclusive additive line values by removing direct child
totals. It SHALL NOT use one subtract-children rule for every metric or use
upstream metric merging for repository aggregation.

#### Scenario: Function contains a nested function
- **WHEN** the upstream metric tree reports a function containing another rated unit
- **THEN** each unit is rated once, the parent's line value excludes the child, and direct complexity follows verified upstream semantics

### Requirement: Ruby analysis is owned
The first usable release SHALL analyze Ruby without routing Ruby source through
`rust-code-analysis`. Ruby classes and modules SHALL be containers; methods,
singleton methods, and lambdas SHALL be units.

#### Scenario: Ruby method contains nested control flow
- **WHEN** an owned Ruby analyzer visits a method containing conditions and loops
- **THEN** it emits one method unit with direct cognitive, cyclomatic, and logical-line measurements and original source lines

#### Scenario: Ruby method contains a lambda
- **WHEN** a method contains a lambda
- **THEN** the lambda is emitted as its own unit and its direct work is not counted again as parent work

### Requirement: Vue analysis covers script and template
The first usable release SHALL analyze Vue single-file components as documents.
It SHALL delegate script and script-setup regions to JavaScript or TypeScript
analysis and SHALL emit a synthetic template unit for template control flow and
inline handlers.

#### Scenario: Vue component has a TypeScript setup script
- **WHEN** a Vue component contains a script-setup region with TypeScript language metadata
- **THEN** its units are analyzed as TypeScript and retain line spans from the original Vue file

#### Scenario: Vue template has branches and loops
- **WHEN** a Vue template contains conditions, loops, conditional expressions, logical branches, or inline handlers
- **THEN** the template unit exposes the contributing measurements and original template span

#### Scenario: Vue component contains styles
- **WHEN** a Vue component contains one or more style regions
- **THEN** those lines contribute to file coverage but do not create debt units

### Requirement: Upstream replacement is language-local
After Ruby and Vue, real usage and profiles SHALL select each next owned language
implementation. A replacement SHALL add compatibility fixtures and performance
evidence before switching one registry entry, without requiring changes to
project, Git, analysis, report, or output crates.

#### Scenario: Owned analyzer replaces an upstream implementation
- **WHEN** the owned analyzer satisfies the language's compatibility fixtures and performance gate
- **THEN** one private registry entry switches while higher-level acceptance output remains unchanged

### Requirement: Unsupported and failed files remain visible
Smackdebt SHALL produce coverage diagnostics for unsupported languages,
unreadable files, oversized files, and parser failures. It MUST NOT treat those
files as healthy source. `.astro` SHALL be a recognized source extension with a
private `Astro` language identity and `astro` machine label, but SHALL remain
explicitly unsupported until a separate change provides document-analysis
fixtures and an analyzer.

Recognized unsupported source SHALL remain selected in codebase, ref-diff, and
worktree-diff inventories. It SHALL contribute to selected and unsupported
coverage and graph-evidence trust, retain its path and source role, and produce
no invented units or dependencies.

#### Scenario: One file cannot be analyzed
- **WHEN** one selected file is unsupported or fails analysis
- **THEN** a codebase report still succeeds with that path, reason, and excluded coverage recorded

#### Scenario: An Astro document is encountered
- **WHEN** discovery supplies a `.astro` source file
- **THEN** it is labeled `astro`, reported as unsupported coverage, and contributes no unit or dependency fact

#### Scenario: An Astro document changes
- **WHEN** an Astro file differs between the current tree and selected ref
- **THEN** both diff inventories retain the file where present and graph evidence cannot treat its source side as fully analyzed

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

### Requirement: Every language provides nesting and parameter fixtures
Every supported language SHALL have exact fixtures for maximum nesting depth and
parameter count before those measurements are consumed. Vue fixtures SHALL cover
script, script-setup, and template regions. A language whose units cannot
declare parameters SHALL have a fixture proving the value is zero.

#### Scenario: A supported language is verified
- **WHEN** its fixture suite runs
- **THEN** exact maximum nesting and parameter values are asserted for representative nested and parameterized units

#### Scenario: A Vue component is verified
- **WHEN** its fixtures run
- **THEN** script, script-setup, and template regions each assert exact nesting and parameter values

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

### Requirement: Anonymous units have safe private match evidence
Each analyzed unit SHALL keep its human display identity separate from the
opaque evidence used to match it across a diff. Analysis SHALL own the value and
export only narrow constructors so the inward-dependent language crate can
write it onto `UnitFact`; its fields and read access SHALL remain private to
analysis. The type MAY therefore appear in the private workspace API snapshot
but SHALL NOT be serialized or expose a dependency from analysis to languages.

A declared function or method SHALL match by its declared identity. An anonymous unit MAY carry a
language-supplied semantic anchor made from its enclosing declared container,
unit kind, and one stable syntax identity: an assignment or binding name; an
enclosing `computed`, `watch`, or callback call plus argument position; a
neighboring literal that identifies the callback; or a Ruby example or context
description.

For remaining anonymous units, the language adapter MAY construct a
deterministic 256-bit BLAKE3 digest and byte length from the unit's exact syntax
bytes while that file's active source buffer exists. Exact syntax SHALL NOT be
retained after the active file worker. Syntax nodes, semantic-anchor values,
digests, and byte lengths SHALL NOT appear in report values, JSON, diagnostics,
terminal output, or logs.

Within one file, comparison SHALL pair units in this order:

1. a declared identity occurring once on each side;
2. a semantic anchor occurring once on each side among remaining units;
3. an exact-syntax digest and byte length occurring once on each side among remaining units.

It SHALL NOT pair by line number, source span, measurement values, rating,
source ordinal, or fuzzy syntax similarity. A shared candidate with a repeated
side is unsafe and SHALL produce one `ambiguous` machine comparison for that
collision group plus one `ambiguous_identity` file diagnostic. A candidate
present on only one side SHALL produce one Added or Removed comparison per unit,
even when repeated. A unit without shared match evidence is likewise one-sided,
not ambiguous. Fingerprint evidence is fixed-size; any repeated fingerprint
bucket is ambiguous rather than guessed, so collision-like evidence cannot pair
a group.

#### Scenario: An unchanged callback moves
- **WHEN** an anonymous callback's exact syntax is unchanged and only its line position moves within its file
- **THEN** its unique semantic anchor or unique exact-syntax digest pairs it and no changed comparison remains

#### Scenario: A callback moves and is edited
- **WHEN** a callback moves and its measurements change while its unique call-site or binding anchor remains
- **THEN** one paired comparison carries the change instead of one added and one removed comparison

#### Scenario: A callback is genuinely added
- **WHEN** an anonymous callback has no before-side declared identity, semantic anchor, or exact-syntax match
- **THEN** it remains an addition rather than being paired by a nearby line, similar measurements, or source order

#### Scenario: An anchor is repeated
- **WHEN** a shared anonymous match candidate occurs twice on either side of one file comparison
- **THEN** the unsafe group remains ambiguous, no debt direction is guessed, and the file carries one ambiguity diagnostic

#### Scenario: A callback is genuinely removed
- **WHEN** an anonymous callback has no after-side declared identity, semantic anchor, or fingerprint match
- **THEN** it remains a removal rather than becoming ambiguous

#### Scenario: A candidate repeats only after the change
- **WHEN** the same candidate occurs twice after the change and not before
- **THEN** two Added comparisons are retained and no ambiguity diagnostic is emitted

#### Scenario: A candidate repeats only before the change
- **WHEN** the same candidate occurs twice before the change and not after
- **THEN** two Removed comparisons are retained and no ambiguity diagnostic is emitted

#### Scenario: Two candidates compete for one
- **WHEN** a shared anchor or fingerprint bucket holds two units on one side and one on the other
- **THEN** one ambiguous collision-group comparison is retained and no added, removed, or paired direction is guessed for that group

#### Scenario: A Ruby example description is stable
- **WHEN** a Ruby example block moves and retains its unique example or context description
- **THEN** that description anchors the unit without exposing parser data across the language seam
