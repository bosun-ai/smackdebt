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

#### Scenario: Two languages import local files differently
- **WHEN** their grammar nodes express equivalent local dependencies
- **THEN** each language implementation emits its own candidates and graph policy receives the same dependency value shape

#### Scenario: Dependency target is dynamic
- **WHEN** source syntax does not identify a safe fixed target
- **THEN** the implementation emits an unresolved reason instead of manufacturing a candidate

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
Architecture fan-in, fan-out, instability, dependency cycles, and coupling SHALL
use parsed `uses` from primary, test, example, and benchmark
source. Module ownership, recovered relations, fixtures, and generated source
SHALL remain context and SHALL NOT enter verdict graphs.

#### Scenario: A cycle exists only through module ownership
- **WHEN** Rust ownership relations form a loop without eligible uses
- **THEN** no dependency-cycle finding is created

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
Analysis SHALL identify an orphan file as a supported primary file with zero
incoming dependencies in the verdict graph that is not an entry file. Entry
files SHALL be exempt, recognized by conventional entry filename and by a
manifest-declared entry. Orphan facts SHALL be descriptive: they SHALL NOT be
rated, SHALL NOT create a finding, and SHALL NOT affect any verdict. Orphan
facts SHALL be derived from existing dependency degree over the file graph
without a new traversal, and SHALL be ordered by data-stable keys.

#### Scenario: Nothing depends on a primary file
- **WHEN** a supported primary file has fan-in zero and is not an entry file
- **THEN** it is recorded as an orphan file without a rating or a finding

#### Scenario: An entry file has no incoming dependency
- **WHEN** a package entry file has fan-in zero
- **THEN** it is not an orphan file

#### Scenario: A non-primary file has no incoming dependency
- **WHEN** a test or fixture file has fan-in zero
- **THEN** it is not recorded as an orphan file
