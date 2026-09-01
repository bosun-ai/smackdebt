## MODIFIED Requirements

### Requirement: JSON version 4 is the machine report contract
`--json` SHALL emit one object with `schema_version: 4`, report mode, selected scope, the
denormalized head defined below, and flat indexed tables for packages, files,
source facts, static relations, history, findings, hotspots, size findings,
orphan files, stable-dependency findings, knowledge-concentration findings,
file change coupling, change-leakage findings, package closures, file reach,
problems, comparisons, health, coverage, and diagnostics. It SHALL NOT emit
a version-3 or version-2 compatibility object, and the CLI SHALL NOT expose a
schema selector before release.

Version 4 SHALL remain the contract while members are added. A member SHALL be
added to version 4 rather than opening a version 5 whenever nothing is removed
and no existing member changes meaning, as when the nested-repository diagnostic
kind, the coverage byte totals, the verdict qualifier, and the problems table
were added.

#### Scenario: A report completes in JSON mode
- **WHEN** stdout is parsed
- **THEN** it contains one version-4 object and no earlier-version compatibility object

#### Scenario: A table is added to the contract
- **WHEN** a new table or head member is serialized without removing or redefining an existing one
- **THEN** it is added to version 4 and no new schema version is introduced

### Requirement: Version 4 serializes problem cards
The report SHALL serialize a `problems` table in problem-rank order, where a
row's position is that card's identity. Each row SHALL expose:

- `pattern`, one of the frozen ids `god_file`, `hub`, `tangle`, `hot_mess`,
  `shotgun_pair`, `bus_risk`, `unstable_dependency`, `measured`,
  `leaky_interface`, `hidden_coupling`;
- `rating`, the card's rating;
- `visibility`, either `default` or `detail`, as a string rather than a
  boolean, because every serialized value is an integer or a string; the value
  is the one the visibility rule `problem-clustering` owns, and this
  specification SHALL NOT restate that rule;
- `anchor`, naming its kind and carrying the file index, file index list,
  package index, or package index pair that kind implies; a `hidden_coupling`
  card SHALL use the existing file-index-list kind for its two files;
- `evidence`, an ordered array preserving the order analysis stored, where each
  item names its kind and carries either an index into the table its kind names
  or an integer value; the kind vocabulary SHALL include an index into
  `change_leakage_findings`, the integer reach of a card's anchor, and the
  integer count of importers that follow a `leaky_interface` card's file, which
  is a stored item like every other and not a rendering-time derivation;
- `claimed`, the findings the card claimed, each naming the table it indexes and
  its position in that table, `change_leakage_findings` included.

No serialized problem value SHALL be a floating-point number or a boolean. Every
index SHALL resolve inside the table it names, no finding SHALL be claimed by two
rows, and no retained finding of any claimable table SHALL be left unclaimed;
validation SHALL fail before exact-byte approval when any of the three holds.

The checked version-4 schema SHALL validate the table, and the schema file SHALL
change in the same slice as the serializer because it uses
`additionalProperties: false` with `required` lists.

#### Scenario: A card is serialized
- **WHEN** any version-4 report is emitted
- **THEN** each `problems` row carries its frozen pattern, rating, visibility string, anchor, ordered evidence, and claimed findings, and validates against the schema

#### Scenario: A detail card is serialized
- **WHEN** a card is `detail` under the rule `problem-clustering` owns, claiming no finding that affects the verdict and no change-leakage finding, and anchoring no rated architecture, coupling, knowledge-concentration, or stable-dependency finding
- **THEN** its `visibility` is the string `detail` and it is present in the machine report at every detail level

#### Scenario: A leakage card is serialized
- **WHEN** a `leaky_interface` or `hidden_coupling` card is emitted
- **THEN** its pattern is the frozen id, its claimed entries index `change_leakage_findings`, and every index resolves

#### Scenario: A follower count is serialized
- **WHEN** a `leaky_interface` card states how many importers follow its file
- **THEN** that count is an evidence item of its own kind carrying an integer, so a machine consumer reads it without counting the card's claimed findings

#### Scenario: An evidence index points outside its table
- **WHEN** acceptance validates the result
- **THEN** it fails before exact-byte approval

#### Scenario: One finding is claimed twice
- **WHEN** two rows claim the same finding of the same table
- **THEN** index-integrity validation fails before exact-byte approval

#### Scenario: One finding is claimed by no row
- **WHEN** a retained finding of a claimable table appears in no row's `claimed`
- **THEN** index-integrity validation fails before exact-byte approval

## ADDED Requirements

### Requirement: Version 4 serializes change leakage and propagation
The report SHALL serialize four additive tables and three additive verdict
members, every value an integer or a string:

- `file_change_coupling`, one row per retained pair, carrying the two file
  indexes with the lower index first, shared commits, union commits, and the
  integer directory distance. Similarity SHALL NOT be serialized, exactly as it
  is not for package coupling.
- `change_leakage_findings`, in the order `change-leakage` defines, each row
  carrying its kind — `leaky_interface` or `hidden_coupling` — an index into
  `file_change_coupling`, and, for a `leaky_interface`, the file index of the
  interface. A row SHALL NOT carry a reference count, a relation kind, a
  resolution outcome, or an edge identity.
- `package_closures`, one row per package whose file-reach value is material,
  with the package index, that package's file count, and the largest number of
  files that transitively depend on one of its files. A package skipped by the
  closure node limit, and a package below the file floor its materiality rule
  names, SHALL each have no row; the table is therefore never a complete package
  index and a consumer SHALL join it by package index rather than by position.
- `file_reach`, one row per candidate file, with the file index and its exact
  repository-wide reach. A file outside the candidate set SHALL have no row, so
  a consumer can never read the table as a complete reach index.

Each `package_graph` row SHALL additionally expose `reach_in`, the number of
packages that transitively depend on that package counting itself.

The `verdict` object SHALL carry `reach`, `core_size`, and `amplification` beside
`tier`, `sentence`, `qualifier`, and `share`, each carrying its integer operands
and omitted entirely when its fact is absent. Reach SHALL carry its source
package or file, and core size SHALL carry its anchor and members. History
coverage SHALL expose the bulk-commit count and the declined-pair count as
integers.

The report SHALL additionally carry graph-evidence status with reason counts,
the selected comparison ref in diff mode, and separate propagation, core, and
change-leakage comparison tables. A diff SHALL also carry current and base graph
evidence separately. For each of the three comparison families it SHALL count
candidate comparisons before evidence filtering, then retain the total withheld
count and the counts that depended on incomplete current and base evidence. The
side counts may overlap when both sides withheld the same candidate. Comparison
rows SHALL retain before and after integer operands, direction, and the package,
file, cycle, or pair that a human row names. Suppressed candidates SHALL remain
absent from finding and comparison tables while the diff graph evidence retains
why they were suppressed.

The diagnostics table's kind vocabulary SHALL include `propagation_skipped`,
carrying the repository-relative path of a package whose file closure exceeded
the node limit. It SHALL be a machine-report disclosure only and SHALL produce
no terminal warning, because it bounds an optional descriptive fact rather than
narrowing what was analyzed.

The checked version-4 schema SHALL validate every addition in the same slice as
the serializer, and every index SHALL resolve inside the table it names.

#### Scenario: A retained pair is serialized
- **WHEN** any version-4 report with history is emitted
- **THEN** each `file_change_coupling` row carries two resolving file indexes with the lower first, integer shared and union commits, a distance of at least 1, and no similarity value

#### Scenario: A leakage finding is serialized
- **WHEN** a `leaky_interface` finding is emitted
- **THEN** its row names its kind, resolves its coupling index, resolves its interface file index, and carries no relation evidence

#### Scenario: A verdict states reach
- **WHEN** a root report of a repository with a material reach is serialized
- **THEN** `verdict.reach` carries both integer counts and the source package while the terminal verdict head states neither

#### Scenario: A diff changes propagation
- **WHEN** a branch changes a material reach value and both graph sides have sufficient evidence
- **THEN** one propagation comparison carries the source identity, direction, and exact before and after operands

#### Scenario: A diff candidate depends on incomplete graph evidence
- **WHEN** a material propagation, core, or change-leakage comparison candidate depends on incomplete current evidence, base evidence, or both
- **THEN** `diff_graph_evidence` identifies each graph side, counts the candidate in its family before filtering, identifies every incomplete side, and the candidate is absent from comparison tables and verdict counts

#### Scenario: A fact is absent
- **WHEN** a scope has no core size, no reach, or no amplification
- **THEN** the corresponding verdict member is absent rather than empty, zero, or null

#### Scenario: A package was too large to close over
- **WHEN** a package exceeds the closure node limit
- **THEN** it has no `package_closures` row and one `propagation_skipped` diagnostic carries its path

#### Scenario: The new tables are scanned for floats
- **WHEN** any acceptance JSON result is validated
- **THEN** no floating-point number and no boolean appears in the four new tables or the three new verdict members
