## MODIFIED Requirements

### Requirement: Architecture integrates with progressive reports
Codebase terminal views SHALL retain the accepted architecture-finding and
closed-witness policy with one grouped resolution warning and no raw relation
rows. Diff terminal default, `--all`, and path views SHALL show only introduced
or removed rated architecture findings with closed witnesses and the grouped
resolution warning when it arises from selected changed relationship facts and
human debt output is present. Current architecture warnings unrelated to the
selected changed facts SHALL remain report and JSON context and SHALL NOT appear
in a diff. A no-debt result SHALL NOT show an architecture warning. JSON version
3 SHALL retain complete relation, graph,
finding, comparison, trust, and resolution tables.
Architecture finding IDs SHALL remain in their own `DebtDiffSelection` list and
SHALL NOT contribute to source `DebtDiffCounts`, `QUALITY`, `AREAS`, or discover
guidance. An architecture-only result SHALL omit those source sections and show
`ARCHITECTURE` independently.

#### Scenario: Diff has introduced and unchanged cycles
- **WHEN** one rated cycle is introduced and another is unchanged
- **THEN** human architecture shows only the introduced finding as Worse while JSON retains both comparisons

#### Scenario: Diff has uncertain resolution only
- **WHEN** no architecture finding is introduced or removed
- **THEN** human architecture detail is absent and a no-debt result omits the architecture warning

#### Scenario: Only an architecture finding changes
- **WHEN** the source and evolution selection lists are empty
- **THEN** `ARCHITECTURE` appears without source `QUALITY`, `AREAS`, `FINDINGS`, discover guidance, or a combined count

### Requirement: Diff compares complete dependency graphs
Diff analysis SHALL compare complete affected before and after graphs, including
unchanged edges required to establish a cycle. Every graph comparison and
ordinary edge change SHALL remain in report and JSON. Human architecture diff
links SHALL include only an introduced rated architecture finding as Worse or a
removed rated architecture finding as Better. An ordinary edge change,
unchanged finding, or other Changed graph context SHALL NOT produce a human
architecture row.

#### Scenario: Changed edge closes an existing path
- **WHEN** one added edge combines with unchanged edges to introduce a package cycle
- **THEN** human output reports the finding as Worse with a complete closed witness while JSON retains the complete graph comparison

#### Scenario: Ordinary edge changes
- **WHEN** edges change without introducing or removing a rated finding
- **THEN** those comparisons remain in JSON and human `ARCHITECTURE` is absent

### Requirement: Static comparisons preserve relation and trust
Ref and worktree diffs SHALL compare relation kind, SourceRole, and trust without
turning context-only changes into architecture verdicts. Complete relation
comparisons SHALL remain report and JSON facts. Human default, `--all`, and path
views SHALL show only introduced or removed rated architecture findings and the
grouped architecture resolution warning, never raw relation changes.

#### Scenario: A diff adds only module ownership
- **WHEN** the current side adds a Rust module declaration without introducing a rated finding
- **THEN** JSON retains the relation comparison and human architecture detail remains absent

#### Scenario: Recovered use changes
- **WHEN** an advisory relation changes without introducing or removing a trusted rated finding
- **THEN** trust remains exact in JSON and no human architecture finding is invented
