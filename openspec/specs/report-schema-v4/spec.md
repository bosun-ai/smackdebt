# report-schema-v4 Specification

## Purpose
TBD - created by archiving change adopt-report-schema-v4. Update Purpose after archive.
## Requirements
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

### Requirement: Version 4 answers the common question first
The object SHALL open with `verdict` and `summary` before its tables. `verdict` SHALL
expose the frozen tier `tier`, its analysis-owned `sentence`, and the report
`mode`. `summary` SHALL expose the checked, high, and watch counts, the debt-diff
counts each labeled by its word, and `worst` as at most three fully resolved
entries carrying a repository-relative path string, identity, and reason. A
consumer SHALL be able to read the verdict and the summary without joining any
table. The head SHALL answer the selected scope, which is the scope the human
report answers for the same invocation and the repository when no path was
selected. Head values SHALL be produced from the same completed report as the
tables they duplicate and SHALL agree with the selected scope's tables.

#### Scenario: A consumer reads only the head
- **WHEN** the first object members are parsed
- **THEN** the tier id, its sentence, the mode, the counts, and up to three resolved worst entries are available with no index lookup

#### Scenario: Head and tables are compared
- **WHEN** acceptance validates a result
- **THEN** every head count and worst entry matches the selected scope's table facts it duplicates

#### Scenario: A path is selected
- **WHEN** the same path selection is rendered as JSON and as the human report
- **THEN** the head states the same tier id, sentence, counts, and worst entries the human verdict states, and not the repository's

#### Scenario: A diff result is emitted
- **WHEN** the report mode is a diff
- **THEN** `verdict` states the diff tier id and sentence and `summary` states each debt-diff count with its word, including zero counts

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

### Requirement: Version 4 locates every retained comparison
Each comparison SHALL expose the source location of the side that still has one, as a
nullable start and end line, so a machine consumer states the same
`path:line` the terminal prints. A comparison that retains no located side
SHALL expose null lines rather than an invented span.

#### Scenario: A regressed unit is serialized
- **WHEN** its comparison row is read
- **THEN** its file index and its start and end lines locate the unit the terminal printed

#### Scenario: A comparison retains no located side
- **WHEN** neither side carries a source span
- **THEN** the row's start and end lines are null

### Requirement: Version 4 serializes exact values only
Serialized values SHALL be integers or strings. `similarity` and `ratio` SHALL NOT be
serialized; their integer operands — shared and union commits, and numerator and
denominator — SHALL remain so a consumer can derive any ratio at its own
precision. No floating-point number SHALL appear in output.

#### Scenario: A coupling row is emitted
- **WHEN** a retained coupling pair is serialized
- **THEN** shared and union commit counts are present and no similarity float is present

#### Scenario: Output is scanned for floats
- **WHEN** any acceptance JSON result is validated
- **THEN** no floating-point number appears anywhere in the object

### Requirement: Version 4 preserves role, trust, and identity contracts
Each file SHALL expose `primary`, `test`, `example`, `benchmark`, `fixture`, or
`generated` SourceRole plus parsed, recovered, or failed outcome, and recovered
units and relations SHALL remain advisory facts excluded from health and verdict
links. The package table SHALL own package ID, repository-relative path, and
current or base presence, with `.` as the machine path for the repository root
and empty or base-only rows remaining valid targets. Static relations SHALL
expose `uses` or `module_ownership` kind separately from SourceRole, parse
trust, resolution outcome, source span, and reference count. File and package
history SHALL expose `touches`, `added_lines`, `deleted_lines`,
`uncounted_changes`, SourceRole, and trust, and coverage SHALL expose
availability, revision, streamed commits, commits containing eligible current
source, mapped eligible changes, mapped context changes, newest and oldest
timestamps, textual changes, uncounted changes, excluded changes, rename gaps,
and reason. Eligible and context history SHALL remain separate after
aggregation.

#### Scenario: A recovered benchmark unit is High
- **WHEN** JSON is emitted
- **THEN** the unit, measurements, role, trust, span, and diagnostic remain present and no health or diff-verdict index counts it

#### Scenario: An empty root package is emitted
- **WHEN** terminal output labels it `repository root`
- **THEN** JSON retains package path `.` and valid references

#### Scenario: Recovered test source imports a module
- **WHEN** its relation is serialized
- **THEN** kind is `uses`, role is `test`, trust is advisory, and it is absent from verdict graph indexes

### Requirement: Version 4 has an executable schema and exact examples
The repository SHALL contain a checked JSON Schema for version 4 and SHALL NOT retain a
version-3 schema as a supported contract. Codebase, clean ref-diff, and mixed
worktree-diff results SHALL validate schema, semantics, indexes, privacy, and
exact bytes from the same invocation. Product documentation and executable
examples SHALL describe version 4 only.

#### Scenario: An index points outside the package table
- **WHEN** acceptance validates the result
- **THEN** it fails before exact-byte approval

#### Scenario: An integration author reads the documentation
- **WHEN** they choose the machine contract
- **THEN** documentation directs them to version 4 and its checked schema

### Requirement: Version 4 discloses discovery and coverage honesty
The diagnostics table's kind vocabulary SHALL include `nested_repository` for a
pruned nested Git checkout and `ambiguous_identity` for an unsafe unit-match
group, each carrying its repository-relative file relation where one exists.
Coverage SHALL expose `selected_files`, `analyzed_files`, `selected_bytes`, and
`unsupported_bytes` as integer totals of the selected scope.

The `verdict` head SHALL carry the analysis-owned incomplete-coverage qualifier
beside `tier` and `sentence` whenever analyzed files are fewer than selected
source files, and SHALL omit it otherwise. The qualifier SHALL require:

- `sentence`, exactly `Not all source was checked.`;
- `detail`, matching `<analyzed> of <selected> source files were analyzed.`;
- integer `selected_files` and `analyzed_files`;
- integer `share_permille` for the retained unsupported byte share;
- `largest_language` when and only when unsupported language bytes identify one.

The file language vocabulary SHALL add `astro`. An Astro file SHALL serialize
with language `astro`, failed trust, unsupported-language diagnostics, and no
invented unit facts. The checked version-4 schema SHALL validate every addition,
and every serialized value SHALL remain an integer or a string. Schema version
SHALL remain 4 because existing members retain their meaning.

#### Scenario: A nested checkout was pruned
- **WHEN** a report over a repository containing a nested checkout is serialized
- **THEN** one diagnostic row carries kind `nested_repository` and the pruned path

#### Scenario: Coverage is serialized
- **WHEN** any version-4 report is emitted
- **THEN** coverage carries integer selected and analyzed file counts and selected and unsupported byte totals that validate against the schema

#### Scenario: An incomplete verdict is serialized
- **WHEN** 7 of 9 selected source files were analyzed
- **THEN** the qualifier carries sentence, detail `7 of 9 source files were analyzed.`, selected_files 9, analyzed_files 7, and share_permille

#### Scenario: A failed-only gap is serialized
- **WHEN** one supported file fails and no language is unsupported
- **THEN** the qualifier has exact file counts and share_permille 0 and omits `largest_language`

#### Scenario: An Astro file is serialized
- **WHEN** a selected `.astro` file is emitted in JSON
- **THEN** its file row has language `astro`, its diagnostic says the language is unsupported, and no unit row refers to it

#### Scenario: A complete verdict is serialized
- **WHEN** analyzed files equal selected source files
- **THEN** the verdict head carries no qualifier member rather than an empty one

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
`tier` and `sentence` for a retained sub-scope of an already completed root
report when that measured root holds High debt. The fact SHALL expose the
sub-scope High count, repository High count, and analysis-owned sentence, with
both counts serialized as integers. The serializer SHALL omit `share` when the
retained sub-scope and root have equal selected-file totals because the root
denominator adds no information.

A fresh explicit file or directory report SHALL omit `share` because limited
discovery did not measure the repository denominator. The checked version-4
schema SHALL continue to allow `share` as an optional verdict member, and the
serializer SHALL omit the member rather than emit an empty or inferred value
whenever measured repository share is unavailable.

#### Scenario: A partial retained sub-scope from a completed root report is serialized
- **WHEN** JSON renders a retained package or directory from a completed root report whose measured root holds High debt and additional selected source outside that sub-scope
- **THEN** the verdict head carries `share` with both integer counts and the same sentence bytes the terminal prints

#### Scenario: A retained sub-scope covers the complete selected inventory
- **WHEN** a completed root report and its retained sub-scope have equal selected-file totals
- **THEN** the verdict head omits `share` because the repository denominator adds no information

#### Scenario: A fresh explicit limited report is serialized
- **WHEN** JSON is requested for a directly selected file or directory whose limited discovery did not measure repository High debt
- **THEN** the verdict head omits `share` rather than treating the selected inventory as the repository denominator

#### Scenario: A root report is serialized
- **WHEN** no path is selected
- **THEN** the verdict head carries no share member rather than an empty or zero one

### Requirement: Version 4 never exposes anonymous match keys
Anonymous semantic anchors, exact-syntax bytes, and their fingerprints SHALL remain
opaque analysis input even though languages construct the analysis-owned value
at the private workspace seam. Version 4 SHALL serialize only the existing human unit
identity, source span, comparison kind, direction, verdict participation,
measurements, ratings, and `ambiguous_identity` diagnostic needed to explain
the result. Each source comparison SHALL carry `participation` as `verdict` or
`context` so its relation to `summary.debt_diff` is explicit. It SHALL NOT add a
match-key, anchor, digest, source fragment, similarity score, or source ordinal
member to any table.

#### Scenario: A moved callback is compared
- **WHEN** a private semantic anchor pairs an anonymous callback across a diff
- **THEN** JSON carries the resulting comparison and no value from the private match key

#### Scenario: A shared match group remains ambiguous
- **WHEN** a plausible anchor or fingerprint bucket has repeated candidates on either side
- **THEN** JSON carries an `ambiguous` comparison and `ambiguous_identity` diagnostic without an anchor, digest, or guessed measurement change

#### Scenario: An unmatched unit exists on one side
- **WHEN** an anonymous unit has no plausible candidate on the other side
- **THEN** JSON carries Added or Removed rather than `ambiguous`

#### Scenario: A source role changes across a diff
- **WHEN** either side of a retained comparison has a fixture or generated role
- **THEN** JSON carries `participation: context`, retains the comparison, and excludes it from `summary.debt_diff`

