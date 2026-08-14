## MODIFIED Requirements

### Requirement: JSON version 3 is the machine report contract
The machine-readable report SHALL identify itself as version 3 and SHALL retain
inventory, package, role, trust, metrics, ratings, static relations, history,
findings, comparisons, health, coverage, diagnostics, complete comparison
links, directions, and counts. Private per-scope `DebtDiffSelection` objects,
their three typed lists, source-only `DebtDiffCounts`, and unique changed-file
warning links SHALL NOT be serialized or added to the JSON Schema. Every
existing version-3 field, meaning, order, and exact byte SHALL remain unchanged.

#### Scenario: Terminal diff selection changes
- **WHEN** the same result is serialized before and after debt-focused presentation
- **THEN** JSON version 3 has identical bytes and contains every complete comparison and count

### Requirement: Version 3 has an executable schema and exact examples
The repository SHALL contain a checked JSON Schema for version 3. Codebase,
clean ref-diff, and mixed worktree-diff results SHALL validate schema, semantics,
indexes, privacy, and exact bytes from the same invocation. Executable report
index validation SHALL also reject a repeated typed ID in any of the three
private owning selection lists without serializing them. Accepted fixtures SHALL
retain exact JSON bytes and SHALL NOT be silently rewritten.

#### Scenario: Private typed selection list repeats an ID
- **WHEN** executable report/index validation inspects the completed source, architecture, or evolution lists
- **THEN** it fails before terminal presentation even though the link is not a JSON field

#### Scenario: Current diff examples pass stronger integrity
- **WHEN** accepted exact examples contain unique trusted selection IDs
- **THEN** their JSON bytes remain unchanged
