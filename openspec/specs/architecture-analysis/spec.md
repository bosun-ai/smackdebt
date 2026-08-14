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
the internal graph.

#### Scenario: Repository uses external packages
- **WHEN** a reference safely identifies an external package but no internal file
- **THEN** the package receives an external dependency count without an internal graph edge

#### Scenario: Some references cannot be resolved
- **WHEN** a codebase report otherwise succeeds
- **THEN** architecture output states exact unresolved and ambiguous counts rather than treating coverage as complete

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
module-ownership relations. JSON version 3 SHALL retain complete relation
tables.

#### Scenario: A repository has many ordinary uses and no cycles
- **WHEN** default terminal output is rendered
- **THEN** architecture states its health without listing arbitrary edges

#### Scenario: A user drills into one package
- **WHEN** incoming, outgoing, ownership, or advisory relations touch it
- **THEN** detailed output shows those relations without unrelated graph rows

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
