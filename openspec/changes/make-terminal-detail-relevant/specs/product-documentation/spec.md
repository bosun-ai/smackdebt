## MODIFIED Requirements

### Requirement: README states JSON detail retention
The README SHALL state that JSON version 3 retains every Watch and High source
finding, all scope summaries and links, complete architecture relationships,
weak and explained history observations, activity, concentration, coverage
fields, diagnostics, and aggregate healthy counts. It SHALL state that terminal
`--all` means all useful debt rather than all retained facts and that JSON is
the complete integration interface.

#### Scenario: Integration author needs complete retained facts
- **WHEN** terminal `--all` omits a raw or contextual family
- **THEN** the README directs the integration author to JSON and explains that healthy units remain aggregate rather than an owned unit index

### Requirement: README documents terminal presentation controls
README and CLI help SHALL describe `--all` as all useful debt. They SHALL state
that a selected path changes scope without enabling raw relationships, weak or
explained history, activity, concentration, or healthy rows. They SHALL describe
JSON as complete and retain the accepted glyph, color, width, redirected-output,
and fallback guidance.

#### Scenario: User asks for more terminal detail
- **WHEN** the user reads help or README guidance for `--all`
- **THEN** it promises every linked useful-debt row without promising raw report facts

### Requirement: Product documentation explains evolutionary signals
The README SHALL explain churn, package change coupling, contributor count,
contributor concentration, and unexplained-coupling Watch findings in simple
product language. It SHALL state that human history shows only actionable
unexplained-coupling findings selected through scope links. Weak and explained
observations, activity, concentration, and history coverage fields SHALL be
documented as JSON-only. Human actionable coupling SHALL end with `not linked in
code`.

#### Scenario: A user interprets evolution output
- **WHEN** the user compares terminal and JSON examples
- **THEN** they can distinguish actionable human history from complete descriptive JSON facts without treating every history value as debt

### Requirement: Documented command examples are checked
The README SHALL associate runnable default and `--all` repository, package,
directory, and file examples with named public dense fixtures, expected status,
and exact output fragments. Examples SHALL cover widths 120, 100, 80, and 50;
limits and useful-debt expansion; closed witnesses; qualifiers; actionable
history ending `not linked in code`; grouped warnings; one discover command;
empty-section absence; one row per unique linked ID in a rendered scope;
excluded-family absence; and JSON completeness. Documentation SHALL explain
that analysis/report owning lists are unique before rendering and that output
does not silently de-duplicate report data.

#### Scenario: Documentation tests run
- **WHEN** documentation validation executes every named example
- **THEN** status, stdout, stderr, sections, rows, commands, and absence rules match the reviewed contract
- **AND** nonempty-line budgets apply only to default codebase and selected path examples, never to `--all`

### Requirement: Documentation covers the unified result
The README SHALL show the same default relevance policy for repository, package,
directory, and file scopes: at most five debt-bearing areas, three source
findings, three architecture findings with closed witnesses, three actionable
history findings, grouped warnings, and one discover command. It SHALL show
`--all` removing only useful-debt limits and SHALL omit empty sections.

#### Scenario: A user follows default and all-detail scope examples
- **WHEN** they move from repository through package and directory to file
- **THEN** scope changes identity and linked rows but does not enable another raw or contextual fact family

### Requirement: README explains recovered advisory evidence
The README SHALL state that retained recovered Watch and High source findings
can appear once in `--all` with advisory trust and non-primary role when present
but do not affect health, default output, architecture verdicts, coupling, or
diff verdicts. Advisory dependency context SHALL remain JSON-only and SHALL NOT
produce raw terminal relationship rows.

#### Scenario: A user sees an advisory source finding
- **WHEN** they compare default, `--all`, path, and JSON output
- **THEN** documentation explains why the source finding appears only in `--all`, while its raw dependency context remains JSON-only

### Requirement: README explains static and history signal rules
The README SHALL distinguish `uses` from `module_ownership`, role from trust,
and actionable findings from descriptive facts. It SHALL state that every human
architecture section contains only architecture findings with closed witnesses,
that the grouped resolution warning is separate, and that raw relations remain
JSON-only. It SHALL document the three-shared-commit and 20% Jaccard threshold
and state that weak and explained coupling remain JSON-only.

#### Scenario: User inspects a package with rich context but no finding
- **WHEN** raw architecture relations and weak history observations exist
- **THEN** documentation explains why terminal sections omit them and JSON retains them
