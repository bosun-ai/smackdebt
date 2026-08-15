## MODIFIED Requirements

### Requirement: Default terminal detail stays concise
The default terminal view SHALL open with the verdict block, SHALL show `AREAS` only when
several debt-bearing child areas exist, and SHALL show at most five such rows.
It SHALL show the first three ranked findings or comparisons within the selected
scope. Zero-value optional facts and empty optional sections SHALL be omitted,
except that a verdict count SHALL always be printed with its word even when it
is zero. When a diff moves no debt, the view SHALL be the verdict line only.

`--all` SHALL show all useful debt without count limits and SHALL NOT show raw
dependency edges, standard-library externals, churn dumps, cyclomatic-1 rows,
weak coupling, or healthy rows; JSON remains the complete view of those facts.

#### Scenario: Selected scope has more than five affected children
- **WHEN** the default terminal report omits child rows
- **THEN** it shows the five most relevant rows without omitted-row bookkeeping

#### Scenario: User requests complete useful terminal detail
- **WHEN** the user supplies `--all`
- **THEN** terminal output shows all useful debt rows, findings, comparisons, and debt-bearing relationships without raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, or healthy rows

#### Scenario: Optional section has no finding
- **WHEN** architecture or history has no actionable finding
- **THEN** the terminal omits that section

#### Scenario: A diff moves no debt
- **WHEN** no comparison or finding counts as debt movement
- **THEN** the view is the verdict line only, with no trailing history, warning, or context section

### Requirement: Codebase summary states rated quality and coverage
The terminal SHALL open with a verdict block stating the selected scope, the analysis-owned
tier sentence, and the counts behind it with every count labeled by its word,
followed by the worst offender with its resolved path and reason when one
exists. It SHALL omit healthy counts, summary ratios, and decorative quality
bars used as data. Coverage gaps SHALL use one grouped warning sentence only
when a gap exists.

#### Scenario: Selection is fully analyzed
- **WHEN** every selected source file is analyzed
- **THEN** the verdict block omits coverage-success text and no warning appears

#### Scenario: Some selected source is excluded
- **WHEN** unsupported or failed files exist
- **THEN** one grouped warning states the gap without treating those files as healthy

#### Scenario: A scope has nothing to check
- **WHEN** the selected scope has zero checked units
- **THEN** the verdict block states the `empty` tier sentence and its zero counts with their words

### Requirement: Terminal layout responds to available width
Terminal rendering SHALL choose each row's shape from that row's own content rather than
from report-level width tiers. A row SHALL stay aligned when its content fits
the resolved width and SHALL otherwise stack its facts on indented lines. Every
ANSI-stripped line SHALL have Unicode display width less than or equal to the
requested width. Measurements, exact counts, cycle witnesses, history commit
evidence, dependency state, commands, finding identity, and comparison identity
SHALL never be silently clipped, and a cycle witness SHALL never be shortened
with an ellipsis. Layout SHALL NOT add bars used as data, summary ratios,
healthy rows, or internal processing facts at any width.

#### Scenario: Wide terminal renders relevant rows
- **WHEN** the resolved width is 120 columns
- **THEN** rows that fit stay aligned and every fact remains visible

#### Scenario: Narrow terminal renders relevant rows
- **WHEN** the resolved width is 50 columns
- **THEN** rows that do not fit stack their facts on indented lines, no line exceeds 50 display cells or breaks a glyph, and no measurement, count, witness, evidence value, command, or identity is lost

#### Scenario: A cycle witness is long
- **WHEN** a cycle witness does not fit the resolved width
- **THEN** the witness stacks across lines and is never shortened with an ellipsis
