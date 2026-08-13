## ADDED Requirements

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
configuration. It SHALL resolve an internal edge only when exactly one candidate
matches and SHALL NOT execute configuration or resolver code.

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
directed package edge for each unique cross-package relationship with exact file
pair and reference counts.

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
unchanged edges required to establish a cycle. An introduced package cycle SHALL
be Worse, a removed package cycle SHALL be Better, and an ordinary edge change
SHALL be Changed unless it creates or removes a rated architecture finding.

#### Scenario: Changed edge closes an existing path
- **WHEN** one added edge combines with unchanged edges to create a package cycle
- **THEN** the diff reports the introduced cycle as Worse with a complete witness

#### Scenario: Changed file is renamed
- **WHEN** Git establishes rename identity before graph comparison
- **THEN** unchanged dependencies are not reported as removed and added solely because the path changed

### Requirement: Architecture integrates with progressive reports
Default and diff terminal reports SHALL show separate architecture sections with
coverage, architecture health, leading affected areas, and concise findings or
changes. Path selection SHALL retain incoming and outgoing facts needed to
explain the selected scope. JSON version 2 SHALL retain the complete root graph.

#### Scenario: User analyzes the repository
- **WHEN** source and dependency analysis complete
- **THEN** one CLI invocation reports code quality and architecture quality in separate explainable sections

#### Scenario: User selects one package
- **WHEN** other packages depend on the selected package
- **THEN** its architecture view includes those incoming relationships without showing unrelated graph regions
