## MODIFIED Requirements

### Requirement: JSON version 4 is the machine report contract
`--json` SHALL emit one object with `schema_version: 4`, report mode, selected scope, the
denormalized head defined below, and flat indexed tables for packages, files,
source facts, static relations, history, findings, hotspots, size findings,
orphan files, stable-dependency findings, knowledge-concentration findings,
problems, comparisons, health, coverage, and diagnostics. It SHALL NOT emit
a version-3 or version-2 compatibility object, and the CLI SHALL NOT expose a
schema selector before release.

Version 4 SHALL remain the contract while members are added. A member SHALL be
added to version 4 rather than opening a version 5 whenever nothing is removed
and no existing member changes meaning, as when the nested-repository diagnostic
kind, the coverage byte totals, and the verdict qualifier were added.

#### Scenario: A report completes in JSON mode
- **WHEN** stdout is parsed
- **THEN** it contains one version-4 object and no earlier-version compatibility object

#### Scenario: A table is added to the contract
- **WHEN** a new table or head member is serialized without removing or redefining an existing one
- **THEN** it is added to version 4 and no new schema version is introduced

### Requirement: Version 4 exposes the new signal tables
The report SHALL serialize a `hotspots` table with file index, maximum unit rating, and
touch count; a `size_findings` table with file index, file or container subject,
container name when the subject is a container, measured value, and triggered
rating; and an `orphan_files` table of file indexes.

Each finding family that owns its own identity type in analysis SHALL own its
own serialized table, so a row's position can never be mistaken for a position
in another family's table. A row's position within its own table SHALL be that
row's stable identity, so another table may refer to it by index; this applies
to `size_findings` the way it already applies to `findings` and
`architecture_findings`. A `stable_dependency_findings` table SHALL carry
`stable_dependency_violation` as its kind together with both packages' integer
degree operands, the reference count, and its witness edges. A
`knowledge_concentration_findings` table SHALL carry `knowledge_concentration`
as its kind together with the package index, SourceRole, trust, contributor
count, numerator, and denominator without identity. `evolutionary_findings`
SHALL carry `unexplained_coupling` as its kind, and `architecture_findings`
SHALL keep `package_cycle` and `file_cycle` as theirs.

History coverage SHALL expose the selected window length in days and the count
of streamed commits excluded by the window. Package records SHALL expose
`manifest_name`, absent when no manifest declares one.

#### Scenario: A hot file is emitted
- **WHEN** a file is a hotspot
- **THEN** its row exposes the file index, its maximum unit rating, and its exact touch count

#### Scenario: A stable dependency violation is emitted
- **WHEN** a stable-dependency violation is serialized
- **THEN** it is a row of the stable-dependency table whose kind names it and whose integer degree operands and reference count are present

#### Scenario: A knowledge concentration finding is emitted
- **WHEN** a knowledge-concentration finding is serialized
- **THEN** it is a row of its own table whose kind names it and whose counts carry no contributor identity

#### Scenario: A size finding is referred to by index
- **WHEN** another table refers to a size finding
- **THEN** it refers to that row's position in `size_findings` and validation resolves it

#### Scenario: A package declares no manifest name
- **WHEN** its record is serialized
- **THEN** `manifest_name` is absent rather than empty or invented

## ADDED Requirements

### Requirement: Version 4 serializes problem cards
The report SHALL serialize a `problems` table in problem-rank order, where a
row's position is that card's identity. Each row SHALL expose:

- `pattern`, one of the frozen ids `god_file`, `hub`, `tangle`, `hot_mess`,
  `shotgun_pair`, `bus_risk`, `unstable_dependency`, `measured`;
- `rating`, the card's rating;
- `visibility`, either `default` or `detail`, as a string rather than a
  boolean, because every serialized value is an integer or a string; the value
  is the one the visibility rule `problem-clustering` owns, and this
  specification SHALL NOT restate that rule;
- `anchor`, naming its kind and carrying the file index, file index list,
  package index, or package index pair that kind implies;
- `evidence`, an ordered array preserving the order analysis stored, where each
  item names its kind and carries either an index into the table its kind names
  or an integer value;
- `claimed`, the findings the card claimed, each naming the table it indexes and
  its position in that table.

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
- **WHEN** a card is `detail` under the rule `problem-clustering` owns, claiming no finding that affects the verdict and anchoring no rated architecture, coupling, knowledge-concentration, or stable-dependency finding
- **THEN** its `visibility` is the string `detail` and it is present in the machine report at every detail level

#### Scenario: An evidence index points outside its table
- **WHEN** acceptance validates the result
- **THEN** it fails before exact-byte approval

#### Scenario: One finding is claimed twice
- **WHEN** two rows claim the same finding of the same table
- **THEN** index-integrity validation fails before exact-byte approval

#### Scenario: One finding is claimed by no row
- **WHEN** a retained finding of a claimable table appears in no row's `claimed`
- **THEN** index-integrity validation fails before exact-byte approval

### Requirement: Version 4 frames a sub-scope verdict
The `verdict` head SHALL carry the analysis-owned repository-share fact beside
`tier` and `sentence` when the selected scope is not the repository root and the
share exists, exposing the scope's High count, the repository's High count, and
the analysis-owned sentence. It SHALL omit the member entirely otherwise, the
way it omits the unsupported-coverage qualifier. Both counts SHALL be integers.

#### Scenario: A sub-scope report is serialized
- **WHEN** a package or directory path is selected and the repository holds High debt
- **THEN** the verdict head carries the share with both integer counts and the same sentence bytes the terminal prints

#### Scenario: A root report is serialized
- **WHEN** no path is selected
- **THEN** the verdict head carries no share member rather than an empty or zero one
