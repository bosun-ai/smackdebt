## MODIFIED Requirements

### Requirement: Codebase scopes expose debt distribution
Each non-file codebase scope SHALL expose exact High and Watch counts for its
children. Each displayed child SHALL expose both its share of selected-scope
debt and its local attention rate across all rated units in that child.

#### Scenario: Debt is spread across child directories
- **WHEN** the selected scope contains several child areas with rated units
- **THEN** each debt-bearing row shows exact High and Watch counts, selected-scope debt share, and local attention rate

#### Scenario: Selected scope has no debt
- **WHEN** the selected scope has zero High and zero Watch units
- **THEN** the terminal states that no child areas need attention and shows no empty distribution table

#### Scenario: Rounded percentages do not sum to one hundred
- **WHEN** independently rounded shares do not total 100 percent
- **THEN** the exact High and Watch counts remain the authoritative values

### Requirement: Codebase child order is severity-led
Default codebase child rows SHALL include only debt-bearing children and sort by
High count descending, Watch count descending, then repository-relative path
ascending. `--all` SHALL retain that order and append healthy-only children in
path order.

#### Scenario: Children have different health distributions
- **WHEN** one child has more High units and another has more total debt units
- **THEN** the child with more High units appears first

#### Scenario: Healthy children exist
- **WHEN** the selected scope contains healthy-only children
- **THEN** the default view reports their count in one quiet summary and `--all` shows their rows

### Requirement: Default terminal detail stays concise
The default terminal view SHALL show at most ten debt-bearing child rows and the
first three retained findings or comparisons within the selected scope. It
SHALL state exact counts for omitted debt-bearing and healthy-only child areas.

#### Scenario: Selected scope has more than ten debt-bearing children
- **WHEN** the default terminal report omits child rows
- **THEN** it states the exact omitted debt-bearing count and does not imply that the visible rows are complete

#### Scenario: User requests complete terminal detail
- **WHEN** the user supplies `--all`
- **THEN** terminal output shows every child row, including healthy-only children, and every retained finding or comparison in the selected scope

### Requirement: Codebase detail follows existing hotspot priority
Non-file codebase views SHALL order retained findings by health, activity,
measurements, path, and source span. Terminal detail SHALL name the measured
signals that reached Watch or High. File views SHALL show every retained finding
with container identity when present and a repository-relative `path:line`
location.

#### Scenario: Finding crosses one limit
- **WHEN** a retained finding reaches Watch or High through one measured signal
- **THEN** terminal detail names that signal and value without presenting healthy signals as reasons

#### Scenario: User drills into a file
- **WHEN** a selected file contains several retained findings
- **THEN** every finding is shown with its rating, attention-causing signals, and copyable source location

### Requirement: Explore points to the next debt-bearing area
The terminal `Explore` command SHALL target the first displayed debt-bearing
child after display filtering and sorting, using its repository-relative path.

#### Scenario: A deeper debt-bearing child exists
- **WHEN** the selected codebase scope has a displayed debt-bearing child
- **THEN** the report prints one valid `smackdebt <path>` command for that first row

#### Scenario: No deeper debt-bearing child exists
- **WHEN** the selected scope is a file or all deeper children are healthy
- **THEN** the report omits the `Explore` section

## ADDED Requirements

### Requirement: Codebase summary states rated quality and coverage
The terminal summary SHALL state the number of rated units, the count and rate
needing attention, and selected files excluded from analysis.

#### Scenario: Selection is fully analyzed
- **WHEN** every selected source file is analyzed
- **THEN** the summary shows zero excluded files and the attention count over all rated units

#### Scenario: Some selected source is excluded
- **WHEN** unsupported or failed files exist
- **THEN** the summary states the exact excluded file count without treating those files as healthy

### Requirement: Coverage notes describe source-analysis gaps
Coverage notes SHALL report selected source files that could not be analyzed.
Routine changed files that are not source candidates SHALL NOT appear as a
coverage problem.

#### Scenario: Diff contains documentation and configuration changes
- **WHEN** a worktree contains changed source and non-source files
- **THEN** the diff analyzes source changes without a coverage note for routine non-source changes
