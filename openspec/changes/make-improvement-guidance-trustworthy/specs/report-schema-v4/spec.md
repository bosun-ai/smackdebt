## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Version 4 never exposes anonymous match keys
Anonymous semantic anchors, exact-syntax bytes, and their fingerprints SHALL remain
opaque analysis input even though languages construct the analysis-owned value
at the private workspace seam. Version 4 SHALL serialize only the existing human unit
identity, source span, comparison kind, direction, measurements, ratings, and
`ambiguous_identity` diagnostic needed to explain the result. It SHALL NOT add a
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
