## ADDED Requirements

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

## MODIFIED Requirements

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
