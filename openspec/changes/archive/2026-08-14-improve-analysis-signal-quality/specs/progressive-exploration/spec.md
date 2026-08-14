## MODIFIED Requirements

### Requirement: Codebase child order is severity-led
Default codebase rows SHALL use the exact finding rank: rating, count of signals
at that rating, total triggered signals, cognitive complexity, cyclomatic
complexity, logical lines, recent activity, then repository-relative path and
source span. Each comparison SHALL be descending except path and span, which
SHALL be ascending.

#### Scenario: Two findings share a rating
- **WHEN** one triggers more signals at that rating
- **THEN** it appears first even when the other has greater recent activity

#### Scenario: Every numeric key ties
- **WHEN** rating, signal counts, metrics, and activity are equal
- **THEN** path and span produce stable order

### Requirement: Codebase detail follows existing hotspot priority
Every displayed finding SHALL show unit kind and SHALL show SourceRole whenever
the role is not primary. Recovered advisory findings SHALL use the same ranking
inside `--all` but SHALL remain absent from default detail.

#### Scenario: A benchmark function is High
- **WHEN** it appears in default detail
- **THEN** its unit kind and benchmark role are visible

#### Scenario: A recovered method is Watch
- **WHEN** detailed output is requested
- **THEN** its unit kind, role when non-primary, and advisory trust are visible

### Requirement: Package summary follows package scopes
Terminal output SHALL render package path `.` as `repository root` in headings,
rows, breadcrumbs, witnesses, and drill guidance. JSON and all machine indexes
SHALL retain `.` unchanged.

#### Scenario: The root is one package
- **WHEN** terminal and JSON render the same report
- **THEN** terminal says `repository root` and JSON package path remains `.`

## ADDED Requirements

### Requirement: Architecture default shows witnesses rather than edge samples
Default architecture output SHALL show architecture health and rated cycle
witnesses only. It SHALL NOT select arbitrary ordinary edges as representative
rows. `--all` and path drill SHALL expose relevant relation detail.

#### Scenario: An acyclic graph has many edges
- **WHEN** default output is rendered
- **THEN** no arbitrary edge list appears
- **AND** detailed output can still inspect the edges

### Requirement: Default terminal sections do not duplicate evidence
A retained fact SHALL appear once in the nearest useful default terminal
section. Source, architecture, and evolution summaries SHALL NOT repeat an
identical finding, relation, coupling pair, history row, or operand already
shown as detail in the same view.

#### Scenario: A coupling finding is the leading evolution fact
- **WHEN** the default report includes its detail
- **THEN** another default section does not repeat the same pair and operands

### Requirement: Renderers consume completed presentation facts
Analysis SHALL own rank keys, verdict inclusion, advisory state, and stable
ordering. Terminal and JSON SHALL read one completed report without
reclassification, trust policy, filesystem, Git, parser, or analysis work.

#### Scenario: One report renders in two formats
- **WHEN** terminal and JSON output are selected in separate runs
- **THEN** role, trust, package, relation, history, and finding facts agree
