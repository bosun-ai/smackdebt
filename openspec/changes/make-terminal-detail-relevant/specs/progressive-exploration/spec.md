## MODIFIED Requirements

### Requirement: Default terminal detail stays concise
Default repository, package, directory, and file views SHALL use the same
selected-scope policy: at most five debt-bearing child areas, three existing
ranked source findings, three architecture findings with closed stored
witnesses, three actionable evolution findings, grouped warnings, and one
discover command. Zero-value optional facts and empty optional sections SHALL
be omitted. A path SHALL select scope without enabling another detail family.

`--all` SHALL remove limits only for debt-bearing child areas, retained linked
Watch/High source findings with existing qualifiers, linked architecture
findings and witnesses, linked actionable evolution findings, and real
failed/recovered diagnostics. It SHALL NOT show healthy, raw relationship,
weak/explained history, activity, concentration, coverage-field, or processing
rows.

#### Scenario: Selected scope has more than five affected children
- **WHEN** the default terminal report omits child rows
- **THEN** it shows the five most relevant debt-bearing rows without omitted-row bookkeeping

#### Scenario: User requests all useful terminal debt
- **WHEN** the user supplies `--all`
- **THEN** terminal output removes useful-debt count limits while every excluded raw, contextual, processing, and healthy family remains absent

#### Scenario: Optional section has no selected finding
- **WHEN** areas, source findings, architecture, history, warnings, or discover guidance selects no row
- **THEN** the terminal omits its heading, table, placeholder, and command

### Requirement: Codebase detail follows existing hotspot priority
Every displayed source finding SHALL follow existing report rank, show unit
kind, and show SourceRole whenever role is not primary. Default output SHALL use
only existing default-verdict finding links. `--all` SHALL additionally include
every scope-linked retained Watch/High advisory finding and SHALL show advisory
trust. Role and trust SHALL remain finding qualifiers and SHALL NOT alter
verdict arithmetic, health indexes, rank, or selection policy.

#### Scenario: A benchmark function is High
- **WHEN** it appears in default detail through an existing verdict link
- **THEN** its unit kind and benchmark role are visible in existing rank order

#### Scenario: A recovered method is Watch
- **WHEN** `--all` selects its retained scope link
- **THEN** its unit kind, non-primary role when present, advisory trust, and finding evidence are visible once

### Requirement: Reports expose evolution as a separate concern
The system SHALL retain history coverage, churn, coupling, concentration, weak
observations, and explained observations in report and JSON. Every human view
SHALL select history rows only through the selected scope's actionable
evolution-finding IDs. Default SHALL show at most three in stable presentation
order; `--all` SHALL show every linked actionable ID. A selected path SHALL NOT
enable any other history table. Each human row SHALL retain shared commits,
union commits, and similarity and SHALL end with `not linked in code`.
Analysis/report aggregation SHALL keep each actionable evolution-finding ID
unique within its owning scope list; terminal output SHALL visit that list once
without identity-based de-duplication.

#### Scenario: Default codebase output has actionable history
- **WHEN** more than three linked actionable coupling findings exist
- **THEN** `HISTORY` shows the first three once with exact evidence and the final `not linked in code` phrase

#### Scenario: User requests all history debt
- **WHEN** a path is selected with `--all`
- **THEN** every linked actionable evolution finding appears once while weak, explained, activity, concentration, coverage-field, and processing rows remain absent

### Requirement: Architecture default shows witnesses rather than edge samples
Every human scope SHALL show `ARCHITECTURE` only for linked architecture
findings. Default SHALL show at most three findings; `--all` SHALL show every
linked architecture finding. Each displayed finding SHALL include one closed
stored witness whose final node returns to its first node. No human default,
`--all`, or path view SHALL show raw resolved edges, ownership, external,
unmatched, ambiguous, incoming, outgoing, or advisory relation rows. The
grouped architecture resolution warning SHALL remain separate.

#### Scenario: An acyclic graph has many retained relations
- **WHEN** a human view has no architecture finding
- **THEN** `ARCHITECTURE` is absent even under `--all` or path selection while JSON retains the relations

#### Scenario: More than three rated cycles exist
- **WHEN** default and `--all` views are rendered
- **THEN** default shows the first three architecture findings with closed witnesses and `--all` shows every linked finding with a closed witness

### Requirement: Terminal layout responds to available width
Terminal rendering SHALL choose compact or stacked row shape from measured
visible content and SHALL preserve every selected useful-debt fact at the
requested Unicode display width. It SHALL NOT create rows for raw architecture,
weak/explained history, activity, concentration, history coverage/processing,
healthy, or other excluded families merely to fill a width-specific layout.
Paths and identities SHALL retain recognizable beginnings and endings, and the
accepted verdict-change safety-fallback rule SHALL remain.

#### Scenario: Dense useful detail is rendered at reviewed widths
- **WHEN** default and `--all` repository, package, directory, and file views render at widths 120, 100, 80, and 50
- **THEN** selected facts fit through compact or stacked layout without an excluded row or safety-fallback shortening
- **AND** only default results at widths 120, 100, and 80 contain at most 60 nonempty lines and only default width-50 results contain at most 100
- **AND** `--all` has no nonempty-line budget and retains every linked useful-debt row

### Requirement: Default terminal sections do not duplicate evidence
Analysis/report aggregation SHALL own unique typed IDs within every scope's
child, source-finding, architecture-finding, evolution-finding, and diagnostic
link list. Index audits SHALL reject a duplicate ID within one owning list.
Terminal default, `--all`, and path selection SHALL trust completed unique lists
and visit each linked ID once without a de-dup set, second collection, identity
mapping, or silent rewrite. A fact linked once by different ancestor scopes
SHALL be permitted to appear once in each separately rendered scope. Within one
rendered scope and owning section, each linked ID SHALL appear once. No empty
heading, empty table, placeholder, or omitted-row count SHALL appear.

#### Scenario: One fact is linked through several ancestors
- **WHEN** ancestor scopes are rendered separately
- **THEN** each scope can show its one linked ID while each individual scope and section shows that ID once

#### Scenario: One owning list repeats a typed ID
- **WHEN** report/index integrity is audited
- **THEN** the audit fails before rendering rather than relying on terminal de-duplication

### Requirement: Renderers consume completed presentation facts
Analysis SHALL own rank keys, verdict inclusion, advisory state, stable order,
and all report links. Terminal selection SHALL iterate the selected scope's
existing child, source-finding, architecture-finding, evolution-finding, and
diagnostic links once without reclassification or role/trust arithmetic. JSON
SHALL read the complete report. Neither renderer SHALL perform filesystem, Git,
parser, analysis, or worker work, and terminal selection SHALL NOT create an
N+1 per-row lookup pattern, de-dup set, second selected-ID collection, or
identity mapping.

#### Scenario: One report renders in two formats
- **WHEN** terminal default, terminal `--all`, path terminal, and JSON output are selected in separate runs
- **THEN** terminal filtering changes only presentation while role, trust, package, relation, history, finding, and index facts agree
