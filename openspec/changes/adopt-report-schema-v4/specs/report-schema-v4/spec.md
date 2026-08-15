## ADDED Requirements

### Requirement: JSON version 4 is the machine report contract
`--json` SHALL emit one object with `schema_version: 4`, report mode, selected scope, the
denormalized head defined below, and flat indexed tables for packages, files,
source facts, static relations, history, findings, hotspots, size findings,
orphan files, stable-dependency findings, knowledge-concentration findings,
comparisons, health, coverage, and diagnostics. It SHALL NOT emit
a version-3 or version-2 compatibility object, and the CLI SHALL NOT expose a
schema selector before release.

#### Scenario: A report completes in JSON mode
- **WHEN** stdout is parsed
- **THEN** it contains one version-4 object and no earlier-version compatibility object

### Requirement: Version 4 answers the common question first
The object SHALL open with `verdict` and `summary` before its tables. `verdict` SHALL
expose the frozen tier `tier`, its analysis-owned `sentence`, and the report
`mode`. `summary` SHALL expose the checked, high, and watch counts, the debt-diff
counts each labeled by its word, and `worst` as at most three fully resolved
entries carrying a repository-relative path string, identity, and reason. A
consumer SHALL be able to read the verdict and the summary without joining any
table. Head values SHALL be produced from the same completed report as the
tables they duplicate and SHALL agree with them.

#### Scenario: A consumer reads only the head
- **WHEN** the first object members are parsed
- **THEN** the tier id, its sentence, the mode, the counts, and up to three resolved worst entries are available with no index lookup

#### Scenario: Head and tables are compared
- **WHEN** acceptance validates a result
- **THEN** every head count and worst entry matches the table facts it duplicates

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
in another family's table. A `stable_dependency_findings` table SHALL carry
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
