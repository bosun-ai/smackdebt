# architecture-analysis Specification

## Purpose
Define dependency extraction, resolution, graph facts, architecture health,
and comparison behavior.
## Requirements
### Requirement: Languages own dependency syntax
Each supported language SHALL translate its grammar-specific dependency forms
into private dependency syntax values during the existing source parse. The
translation SHALL include reference kind, target text, source span, and ordered
resolution candidates, and SHALL perform no filesystem or Git access.

A grammar form that imports several items in one declaration — such as a Rust
`use` declaration with a brace list — SHALL emit one reference per imported
item with that item's full reconstructed path, so item-level dependency
fidelity is preserved. A grouped declaration SHALL NOT collapse to one
reference naming only its shared prefix.

#### Scenario: Two languages import local files differently
- **WHEN** their grammar nodes express equivalent local dependencies
- **THEN** each language implementation emits its own candidates and graph policy receives the same dependency value shape

#### Scenario: Dependency target is dynamic
- **WHEN** source syntax does not identify a safe fixed target
- **THEN** the implementation emits an unresolved reason instead of manufacturing a candidate

#### Scenario: One declaration imports three items
- **WHEN** a Rust file contains `use crate::{a, b, c};`
- **THEN** extraction emits three references, one per item, and no single reference for the bare prefix

### Requirement: Project resolves dependencies from repository facts
Project orchestration SHALL resolve dependency candidates against one read-only
index of discovered files, package identities, and supported project
configuration. It SHALL resolve an internal edge only when exactly one
candidate matches and SHALL NOT execute configuration or resolver code.

#### Scenario: One candidate exists
- **WHEN** a dependency candidate identifies exactly one discovered internal file
- **THEN** the result links the source and target file identities

#### Scenario: Several candidates exist
- **WHEN** more than one internal file remains plausible
- **THEN** the reference is ambiguous and no internal edge is guessed

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

### Requirement: Dependency graphs store each relationship once
The report SHALL own one deduplicated edge for each directed internal file pair
with exact reference count and representative locations. It SHALL derive one
directed package edge for each unique cross-package relationship with exact
file-pair and reference counts.

#### Scenario: Several imports link the same two files
- **WHEN** the source file refers to the target file more than once
- **THEN** one file edge records the reference count without repeating graph ownership

#### Scenario: Several files link the same two packages
- **WHEN** several unique file pairs cross the same package boundary
- **THEN** one package edge records their file-pair and reference counts

### Requirement: Graph algorithms expose exact static structure
Analysis SHALL calculate strongly connected components, stable cycle witnesses,
unique fan-in, unique fan-out, and instability from flat indexed graphs without
parser, filesystem, Git, or output access. Instability SHALL equal outgoing
neighbors divided by incoming plus outgoing neighbors and SHALL be absent when
both counts are zero.

#### Scenario: Package has incoming and outgoing neighbors
- **WHEN** its unique fan-in is three and unique fan-out is one
- **THEN** its instability is one quarter and the exact neighbor counts remain available

#### Scenario: Several cycle witnesses are possible
- **WHEN** a strongly connected component contains several paths
- **THEN** stable path ordering selects the same concise witness in serial and parallel runs

### Requirement: Architecture health rates cycles separately
A dependency cycle crossing package responsibility SHALL be a High architecture
finding. A file cycle contained within one package SHALL be a Watch architecture
finding. Degree and instability SHALL remain descriptive facts. Code health and
architecture health SHALL retain separate counts and findings.

#### Scenario: Healthy functions form a package cycle
- **WHEN** two packages depend on each other and all rated source units are Healthy
- **THEN** code health remains Healthy while architecture reports one High cycle finding

#### Scenario: Package has high fan-out without a cycle
- **WHEN** it has many outgoing neighbors but no rated architecture rule applies
- **THEN** output shows the exact fan-out without labeling it Watch or High

### Requirement: Diff compares complete dependency graphs
Diff analysis SHALL compare complete affected before and after graphs, including
unchanged edges required to establish a cycle. An introduced package cycle
SHALL be Worse, a removed package cycle SHALL be Better, and an ordinary edge
change SHALL be Changed unless it creates or removes a rated architecture
finding.

#### Scenario: Changed edge closes an existing path
- **WHEN** one added edge combines with unchanged edges to create a package cycle
- **THEN** the diff reports the introduced cycle as Worse with a complete witness

#### Scenario: Changed file is renamed
- **WHEN** Git establishes rename identity before graph comparison
- **THEN** unchanged dependencies are not reported as removed and added solely because the path changed

### Requirement: Architecture integrates with progressive reports
Default terminal reports SHALL show architecture health totals and rated cycle
witnesses without arbitrary dependency-edge rows. `--all` and path drill SHALL
show relevant incoming, outgoing, unresolved, ambiguous, advisory, and
module-ownership relations. The machine report SHALL retain complete relation
tables in whichever schema version is current, so retiring one version never
retires the relation tables.

#### Scenario: A repository has many ordinary uses and no cycles
- **WHEN** default terminal output is rendered
- **THEN** architecture states its health without listing arbitrary edges

#### Scenario: A user drills into one package
- **WHEN** incoming, outgoing, ownership, or advisory relations touch it
- **THEN** detailed output shows those relations without unrelated graph rows

#### Scenario: A machine consumer reads relations
- **WHEN** the current machine report is parsed
- **THEN** every dependency edge, package edge, external dependency, resolution diagnostic, and package-graph row remains present

### Requirement: Static relation kind is independent from evidence
Every extracted static relation SHALL have kind `uses` or `module_ownership`.
SourceRole, parse trust, resolution outcome, span, and reference count SHALL be
separate evidence fields and SHALL NOT become additional relation kinds.

#### Scenario: Test source uses another module
- **WHEN** a parsed test file resolves an import
- **THEN** the relation kind is `uses` and its separate source role is `test`

#### Scenario: Recovered source declares a module
- **WHEN** recovered Rust syntax resolves an external module declaration
- **THEN** the relation kind is `module_ownership` and its trust is advisory

### Requirement: Rust module declarations express module ownership
Rust external `mod child;` declarations SHALL emit `module_ownership`. Inline
modules SHALL remain within the declaring file. Imports, qualified paths, and
safely resolved macro paths SHALL emit `uses`.

#### Scenario: A Rust file declares an external child
- **WHEN** the resolver identifies the child module file
- **THEN** one module-ownership relation links parent and child

#### Scenario: A Rust file imports a child symbol
- **WHEN** the resolver identifies the referenced repository module
- **THEN** one uses relation retains the import evidence

### Requirement: Verdict graphs use trusted eligible uses only
Architecture verdict graphs SHALL use parsed, trusted `uses` relations whose
source role is primary. The verdict graphs SHALL be the package dependency
edges, the package dependency cycle graph, the file dependency cycle graph, the
package graph's fan-in, fan-out, and instability measurements, and the
stable-dependency comparison derived from them. Trusted `uses` relations from
test, example, and benchmark source SHALL remain complete in the machine report
as context and SHALL NOT create, remove, or change an architecture verdict.
Module ownership, recovered relations, fixtures, and generated source SHALL
remain context and SHALL NOT enter verdict graphs.

The file dependency cycle graph SHALL additionally exclude a `uses` relation
between two files when a `module_ownership` relation exists between the same
unordered file pair in either direction, because a module declaration and the
imports that accompany it are one wiring relationship rather than a cycle. The
exclusion SHALL be pairwise: only relations between that owning pair SHALL be
excluded, and every other relation of the same strongly connected component SHALL
remain. The witness selection for a reported cycle SHALL apply the identical
exclusion, so witnesses and detected cycles cannot disagree. The exclusion SHALL
apply to the cycle graph only and SHALL NOT change fan-in, fan-out, instability,
or orphan facts.

Explaining package change coupling SHALL continue to use trusted eligible `uses`
relations of the roles primary, test, example, and benchmark, so a dependency
that never enters a verdict graph still explains why two packages change
together.

#### Scenario: Rust module wiring never forms a file cycle
- **WHEN** a Rust parent declares `mod child;` and the same pair also imports each other through trusted primary uses
- **THEN** the ownership relations never enter the cycle graph, the uses relations between that owning pair are excluded from it as well, no file-cycle finding is created, and every relation stays visible in the machine report

#### Scenario: A cycle through unowned files survives the exclusion
- **WHEN** two sibling modules import each other, or a cycle merely passes through an owning pair by way of other files
- **THEN** the cycle is still detected and its witnesses are the relations that remain in the graph

#### Scenario: A module declared only under a test configuration creates no package cycle
- **WHEN** a primary Rust file declares `#[cfg(test)] mod tests;` and the declared file imports another package that depends back on this one
- **THEN** the declared file is test source, its relations stay in the machine report, and no package dependency cycle finding is created

#### Scenario: A dev-dependency test import creates no package cycle
- **WHEN** two packages depend on each other only because a test file or a `#[cfg(test)]` module in one imports the other
- **THEN** no package dependency cycle finding and no stable-dependency finding is created, and both test-role relations remain present in the machine report

#### Scenario: A recovered use closes a cycle
- **WHEN** parsed uses form an acyclic graph and one advisory use would close it
- **THEN** the advisory relation remains visible but no cycle verdict is created

### Requirement: Static comparisons preserve relation and trust
Ref and worktree diffs SHALL compare relation kind, SourceRole, and trust without
turning context-only changes into architecture verdicts.

#### Scenario: A diff adds only module ownership
- **WHEN** the current side adds a Rust module declaration without adding uses
- **THEN** detailed output reports the relation change
- **AND** architecture health is not Worse

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

### Requirement: Stable dependency violations are Watch findings
Analysis SHALL create a Watch architecture finding when a package depends on a
package that is less stable than itself and the depending package has at least 2
references into the depended-on package. A package is less stable than another
when its instability, unique fan-out over unique neighbor total, is greater.
Comparison SHALL use integer
cross-multiplication of the degree operands and SHALL NOT compare floating-point
instability values: for a depending package with unique fan-out `outA` and
neighbor total `totalA` and a depended-on package with `outB` and `totalB`, a
violation exists only when `outB * totalA` is greater than `outA * totalB`.
Equal cross products SHALL NOT be a violation. A package with no neighbors SHALL
NOT participate. The finding SHALL retain both packages' exact degree operands
and the reference count. Degree and instability SHALL remain descriptive facts
outside this finding.

#### Scenario: A stable package depends on an unstable one
- **WHEN** a package with lower instability has 3 references into a package with higher instability
- **THEN** one Watch architecture finding retains both packages' fan-in, fan-out, and the reference count

#### Scenario: The dependency is incidental
- **WHEN** the same relationship exists with exactly 1 reference
- **THEN** no finding is created and the degree facts remain descriptive

#### Scenario: Instability is equal
- **WHEN** the integer cross products of the two packages are equal
- **THEN** no violation exists

### Requirement: Orphan files are descriptive facts
Analysis SHALL identify an orphan file as a supported primary file that is not an
entry file and has zero incoming trusted eligible uses, where trusted eligible
uses are parsed `uses` relations from primary, test, example, and benchmark
source. Entry files SHALL be exempt, recognized by conventional entry filename
and by a manifest-declared entry. Orphan facts SHALL be descriptive: they SHALL
NOT be rated, SHALL NOT create a finding, and SHALL NOT affect any verdict.
Orphan facts SHALL be derived from existing dependency degree over the file graph
without a new traversal, and SHALL be ordered by data-stable keys.

#### Scenario: Nothing depends on a primary file
- **WHEN** a supported primary file has zero incoming trusted eligible uses and is not an entry file
- **THEN** it is recorded as an orphan file without a rating or a finding

#### Scenario: A primary file imported only by tests is not an orphan
- **WHEN** the only incoming trusted eligible uses of a primary file come from test, example, or benchmark source
- **THEN** it is not recorded as an orphan file, even though those relations do not enter a verdict graph

#### Scenario: An entry file has no incoming dependency
- **WHEN** a package entry file has fan-in zero
- **THEN** it is not an orphan file

#### Scenario: A non-primary file has no incoming dependency
- **WHEN** a test or fixture file has fan-in zero
- **THEN** it is not recorded as an orphan file

### Requirement: A Rust module file owns the directory named after it
Project resolution SHALL read a relative candidate of a Rust source file against
the directory that file's own modules live in: the file's own directory when the
file is `mod.rs`, `lib.rs`, or `main.rs`, and a directory named after the file
otherwise. That reading SHALL take precedence over the sibling reading of the
same candidate wherever it matches a discovered file, and the sibling reading
SHALL remain for every candidate it does not. Resolution SHALL still link an
internal edge only when exactly one file matches.

#### Scenario: A module declared inside a plain module file
- **WHEN** `a.rs` declares `mod child;` and the repository contains `a/child.rs`
- **THEN** the declaration resolves to `a/child.rs` rather than to a sibling `child.rs`

#### Scenario: A parent reference inside a plain module file
- **WHEN** `a/child.rs` contains `use super::sibling::work;`
- **THEN** the reference resolves against `a`'s own parent module rather than one directory higher

#### Scenario: A module declared inside a directory module file
- **WHEN** `a/mod.rs`, `lib.rs`, or `main.rs` declares `mod child;`
- **THEN** the declaration resolves beside the declaring file, unchanged

### Requirement: Rust cfg(test) scope assigns test role to references
Rust dependency extraction SHALL record whether a reference is declared in a
test-only scope. A reference SHALL be test-scoped when the item that declares it,
or any ancestor module of that item, carries an outer attribute whose path is
exactly `cfg` and whose token tree contains the identifier `test` at any depth
that is not inside a token tree immediately following the identifier `not`.
`#[cfg(test)]`, `#[cfg(all(test, not(loom)))]`, and `#[cfg(any(test, fuzzing))]`
SHALL be test-scoped. `#[cfg(not(test))]`, `#[cfg(feature = "test")]`, and
`#[cfg_attr(test, ...)]` SHALL NOT be test-scoped. The rule SHALL be syntactic:
it SHALL NOT evaluate configuration predicates, read enabled features, or
consult build configuration.

A test-scoped reference SHALL carry the source role that is the later of its
file's role and `test`, so a reference declared in a primary file becomes `test`
while a reference in a file whose role is already test, example, benchmark,
fixture, or generated keeps that role. Scope SHALL remain evidence: it SHALL NOT
create a relation kind and SHALL NOT change resolution, trust, coverage
partitioning, or external dependency counts. Where one file both declares a
default-scoped and a test-scoped reference to the same target, the machine report
SHALL retain one relation per role rather than merging them into one.

#### Scenario: An inline test module inside production source
- **WHEN** a primary Rust file declares `#[cfg(test)] mod tests` whose imports resolve to another repository file
- **THEN** those relations carry the source role `test` while the same file's other imports stay primary

#### Scenario: A composite configuration predicate still selects test
- **WHEN** an item is gated by `#[cfg(all(test, not(loom)))]` or by `#[cfg(any(test, fuzzing))]`
- **THEN** its references are test-scoped

#### Scenario: A configuration predicate that is not test scope
- **WHEN** an item is gated by `#[cfg(not(test))]`, by `#[cfg(feature = "test")]`, or by `#[cfg_attr(test, ...)]`
- **THEN** its references are not test-scoped and keep their file's role

#### Scenario: A non-primary file contains a test scope
- **WHEN** a fixture or generated file declares a `#[cfg(test)]` module
- **THEN** its references keep the file's own role rather than being promoted or demoted to `test`

#### Scenario: One file pair carries both roles
- **WHEN** a primary file imports a target directly and imports it again inside a `#[cfg(test)]` module
- **THEN** the machine report retains a primary relation and a test relation for that pair, and the relation kind of both is `uses`
