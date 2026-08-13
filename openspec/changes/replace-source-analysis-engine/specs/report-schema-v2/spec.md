## ADDED Requirements

### Requirement: JSON version 2 is the machine report contract
`--json` SHALL emit one top-level object with `schema_version: 2`, report mode,
selected scope, and flat indexed tables for paths, scopes, files, code findings,
comparisons, health, activity, and diagnostics. The CLI SHALL NOT expose a JSON
version selector before the first public release.

#### Scenario: Integration requests a codebase report
- **WHEN** the CLI completes codebase analysis with `--json`
- **THEN** standard output contains one valid schema-version-2 document and no version-1 compatibility object

#### Scenario: Integration requests a diff report
- **WHEN** the CLI completes ref comparison with `--json`
- **THEN** the same version-2 document shape contains before and after code comparison facts

### Requirement: Version 2 keeps identities and facts in one place
Each repository-relative path, file, retained code finding, diagnostic, and comparison SHALL
have one owner in its table. Other tables SHALL refer to those
facts through stable typed indexes rather than copying related values.

#### Scenario: Finding contributes to several scopes
- **WHEN** one code finding appears in file, directory, package, and repository views
- **THEN** the finding and path are stored once and each applicable scope refers to them by index

### Requirement: Version 2 remains explainable and complete
Version 2 SHALL retain exact measurement values, signal ratings, source spans,
coverage failures, every Watch and High code finding, and every diff comparison.
Healthy units SHALL remain aggregate counts.

#### Scenario: Terminal output omits detail
- **WHEN** default terminal limits hide rows or findings
- **THEN** JSON version 2 still contains every retained source fact

### Requirement: Version 2 has an executable schema
The repository SHALL contain a checked JSON Schema for version 2. Black-box JSON
fixtures SHALL validate against that schema and exact reviewed snapshots.

#### Scenario: Serializer changes a field type
- **WHEN** an emitted version-2 field no longer matches its checked schema
- **THEN** the black-box schema validation fails before snapshot review
