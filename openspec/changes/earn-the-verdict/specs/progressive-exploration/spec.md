## MODIFIED Requirements

### Requirement: Default terminal detail stays concise
The default terminal view SHALL open with the verdict block, SHALL show `AREAS` only when
several debt-bearing child areas exist, and SHALL show at most five such rows.
It SHALL show the first three ranked findings or comparisons within the selected
scope. Zero-value optional facts and empty optional sections SHALL be omitted,
except that a verdict count SHALL always be printed with its word even when it
is zero. When a diff moves no debt, the view SHALL be the verdict line only.

`--top N` SHALL provide a middle level of detail between the default and
`--all`: it SHALL raise or lower only the number of displayed ranked findings
or comparisons to `N`, leaving the architecture and history section limits
unchanged. `N` SHALL be a positive integer, and `--top` SHALL conflict with
`--json` and with `--all`.

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

#### Scenario: A middle finding limit is requested
- **WHEN** the user supplies `--top 10` and twelve ranked findings exist
- **THEN** ten findings are shown while architecture and history sections keep their default limits

#### Scenario: The limit exceeds the findings
- **WHEN** the user supplies `--top 10` and four ranked findings exist
- **THEN** all four are shown and no filler or bookkeeping row appears

### Requirement: Codebase summary states rated quality and coverage
The terminal SHALL open with a verdict block stating the selected scope, the analysis-owned
tier sentence, and the counts behind it with every count labeled by its word,
followed by the worst offender with its resolved path and reason when one
exists. When the verdict carries an unsupported-coverage qualifier, the
qualifier row SHALL render inside the verdict block directly under the tier
sentence, using the analysis-owned qualifier bytes. It SHALL omit healthy
counts, summary ratios, and decorative quality bars used as data. Coverage
gaps SHALL use one grouped warning sentence only when a gap exists.

#### Scenario: Selection is fully analyzed
- **WHEN** every selected source file is analyzed
- **THEN** the verdict block omits coverage-success text and no warning appears

#### Scenario: Some selected source is excluded
- **WHEN** unsupported or failed files exist
- **THEN** one grouped warning states the gap without treating those files as healthy

#### Scenario: A scope has nothing to check
- **WHEN** the selected scope has zero checked units
- **THEN** the verdict block states the `empty` tier sentence and its zero counts with their words

#### Scenario: The verdict is qualified
- **WHEN** the unsupported byte share exceeds the qualifier threshold
- **THEN** the qualifier row appears under the tier sentence inside the verdict block
