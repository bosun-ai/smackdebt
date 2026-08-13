## ADDED Requirements

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

## MODIFIED Requirements

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
