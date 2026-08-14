## ADDED Requirements

### Requirement: JSON version 3 is the machine report contract
`--json` SHALL emit one object with `schema_version: 3`, report mode, selected
scope, and flat indexed tables for packages, files, source facts, static
relations, history, findings, comparisons, health, coverage, and diagnostics.
The CLI SHALL NOT expose a schema selector before release.

#### Scenario: A report completes in JSON mode
- **WHEN** stdout is parsed
- **THEN** it contains one version-3 object and no version-2 compatibility object

### Requirement: Version 3 exposes SourceRole and trust
Each file SHALL expose `primary`, `test`, `example`, `benchmark`, `fixture`, or
`generated` SourceRole plus parsed, recovered, or failed outcome. Recovered
units and relations SHALL remain advisory facts while health and verdict links
exclude them.

#### Scenario: A recovered benchmark unit is High
- **WHEN** JSON is emitted
- **THEN** the unit, metrics, role, trust, span, and diagnostic remain present
- **AND** no health or diff-verdict index counts it

### Requirement: Version 3 owns stable package identity
The package table SHALL own package ID, repository-relative path, and current or
base presence. Empty and base-only rows SHALL remain valid targets. The machine
path for repository root SHALL remain `.`.

#### Scenario: An empty root package is emitted
- **WHEN** terminal labels it `repository root`
- **THEN** JSON retains package path `.` and valid references

### Requirement: Version 3 separates relation kind from evidence
Static relations SHALL expose `uses` or `module_ownership` kind separately from
SourceRole, parse trust, resolution outcome, source span, and reference count.
Advisory and verdict graph links SHALL be distinguishable.

#### Scenario: Recovered test source imports a module
- **WHEN** its relation is serialized
- **THEN** kind is `uses`, role is `test`, trust is advisory, and it is absent from verdict graph indexes

### Requirement: Version 3 exposes exact history fields
File and package history SHALL expose `touches`, `added_lines`, `deleted_lines`,
`uncounted_changes`, SourceRole, and trust. Coverage SHALL expose availability,
revision, streamed commits, commits containing eligible current source, mapped
eligible changes, mapped context changes, newest and oldest timestamps, textual
changes, uncounted changes, excluded changes, rename gaps, and reason. Coupling
SHALL expose package IDs, endpoint SourceRole and trust for descriptive rows,
`shared_commits`, `union_commits`, and similarity. Concentration SHALL expose
package ID, SourceRole, trust, contributor count, numerator, denominator, and
ratio. Eligible and context history SHALL remain separate after aggregation.

#### Scenario: A weak coupling row is not a default finding
- **WHEN** it misses the finding threshold
- **THEN** JSON retains its exact operands without a finding link

#### Scenario: A current generated change is mapped
- **WHEN** history maps a change to a current generated file
- **THEN** it increments context changes and its generated trusted history row rather than excluded changes or an eligible row

### Requirement: Version 3 has an executable schema and exact examples
The repository SHALL contain a checked JSON Schema for version 3. Codebase,
clean ref-diff, and mixed worktree-diff results SHALL validate schema, semantics,
indexes, privacy, and exact bytes from the same invocation.

#### Scenario: An index points outside the package table
- **WHEN** acceptance validates the result
- **THEN** it fails before exact-byte approval

### Requirement: Version 2 is retired before first release
Product documentation, executable examples, and acceptance SHALL describe
version 3 only. Archived history MAY retain version 2, but current CLI behavior
SHALL NOT present it as supported.

#### Scenario: An integration author reads the README
- **WHEN** they choose the machine contract
- **THEN** documentation directs them to version 3 and its schema
