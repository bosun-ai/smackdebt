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

### Requirement: Codebase scopes expose debt distribution
Each non-file codebase scope SHALL expose exact High and Watch counts for its children.
Terminal area rows SHALL state those counts in words and SHALL NOT add a share
percentage, an attention rate, or a rate bar, because a bar used as data and a
ratio the reader cannot check were removed from human output. A machine
consumer SHALL derive any share from the exact counts the report already
serializes.

#### Scenario: Debt is spread across child directories
- **WHEN** the selected scope contains several child areas with rated units
- **THEN** each debt-bearing row shows its exact High and Watch counts labeled by their words and no share, rate, or bar

#### Scenario: Selected scope has no debt
- **WHEN** the selected scope has zero High and zero Watch units
- **THEN** no area section is written

### Requirement: Diff scopes expose change distribution
Each diff scope SHALL expose exact Worse, Better, and Changed counts. Terminal area rows
SHALL state those counts in words and SHALL NOT add a share percentage or a
share bar, for the same reason the codebase area rows do not. A machine
consumer SHALL derive any share from the exact counts the report already
serializes.

#### Scenario: Changed units span several directories
- **WHEN** the selected diff scope has changes in several child areas
- **THEN** each row shows its exact direction counts labeled by their words and no share or bar

#### Scenario: Diff selection has no retained comparisons
- **WHEN** the selected scope moved no debt
- **THEN** no area section is written

### Requirement: Explore points to the next debt-bearing area
The terminal discover line SHALL target the first displayed debt-bearing child after
display filtering and sorting, using its repository-relative path. The line
SHALL be the word `next:` followed by `smackdebt <path>`, decorated with U+F46B
before the word when decoration is enabled, so the line reads without its
glyph.

#### Scenario: A deeper debt-bearing child exists
- **WHEN** the selected codebase scope has a displayed debt-bearing child
- **THEN** the report prints `next: smackdebt <path>` for that first row

#### Scenario: No deeper debt-bearing child exists
- **WHEN** the selected scope is a file or all deeper children are healthy
- **THEN** the report omits the discover line
