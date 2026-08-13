## MODIFIED Requirements

### Requirement: Codebase scopes expose debt distribution
Each non-file codebase scope SHALL expose exact High and Watch counts for its
children. Each displayed child SHALL expose both its share of selected-scope
debt and its local attention rate across all rated units in that child. Terminal
area views SHALL add a rate bar whose non-zero value remains visible.

#### Scenario: Debt is spread across child directories
- **WHEN** the selected scope contains several child areas with rated units
- **THEN** each debt-bearing row shows exact High and Watch counts, selected-scope debt share, local attention rate, and a rate bar

#### Scenario: Selected scope has no debt
- **WHEN** the selected scope has zero High and zero Watch units
- **THEN** the terminal states that no child areas need attention and shows no empty distribution table

#### Scenario: Small non-zero rate is rendered
- **WHEN** a child attention rate is greater than zero but smaller than one whole bar cell
- **THEN** the rate bar contains a fractional filled block

### Requirement: Default terminal detail stays concise
The default terminal view SHALL show at most ten debt-bearing child rows and the
first three retained findings or comparisons within the selected scope. It
SHALL state exact counts for omitted debt-bearing and healthy-only child areas.
Zero-value optional facts SHALL be omitted when their absence does not change
the meaning.

#### Scenario: Selected scope has more than ten debt-bearing children
- **WHEN** the default terminal report omits child rows
- **THEN** it states the exact omitted debt-bearing count and does not imply that the visible rows are complete

#### Scenario: User requests complete terminal detail
- **WHEN** the user supplies `--all`
- **THEN** terminal output shows every child row, including healthy-only children, and every retained finding or comparison in the selected scope

#### Scenario: Optional count is zero
- **WHEN** a finding has zero recent touches or every selected file was analyzed
- **THEN** the terminal omits the zero activity or excluded-file phrase

### Requirement: Codebase detail follows existing hotspot priority
Non-file codebase views SHALL order retained findings by health, activity,
measurements, path, and source span. Terminal detail SHALL present a severity
identity, an unbroken repository-relative `path:line` location, and only the
measured signals that reached Watch or High. File views SHALL show every
retained finding with container identity when present.

#### Scenario: Finding crosses one limit
- **WHEN** a retained finding reaches Watch or High through one measured signal
- **THEN** terminal detail names that signal and value without presenting healthy signals as reasons

#### Scenario: User drills into a file
- **WHEN** a selected file contains several retained findings
- **THEN** every finding is shown with its rating, attention-causing signals, and copyable source location

### Requirement: Diff scopes expose change distribution
Each diff scope SHALL expose exact Worse, Better, and Changed counts. A child's
share SHALL equal its sum of those counts divided by the selected scope's sum.
Terminal area views SHALL visualize that share without replacing exact counts.

#### Scenario: Changed units span several directories
- **WHEN** the selected diff scope has changes in several child areas
- **THEN** each row shows exact direction counts, its nearest-whole-percent share of selected changed units, and a share bar

#### Scenario: Diff selection has no retained comparisons
- **WHEN** the selected scope has a zero change-count denominator
- **THEN** every child share is `0%` and its bar is empty

### Requirement: Codebase summary states rated quality and coverage
The terminal summary SHALL state the number of rated units, the count and rate
needing attention, High, Watch, and Healthy counts, and selected source
coverage. A neutral attention bar SHALL reinforce the exact attention ratio
without presenting one repository score.

#### Scenario: Selection is fully analyzed
- **WHEN** every selected source file is analyzed
- **THEN** the summary confirms all selected source files were analyzed and omits an excluded-file phrase

#### Scenario: Some selected source is excluded
- **WHEN** unsupported or failed files exist
- **THEN** the summary states the exact excluded file count without treating those files as healthy

## ADDED Requirements

### Requirement: Terminal layout responds to available width
Terminal rendering SHALL use a full aligned table at 100 columns or more, a
compact aligned table from 70 through 99 columns, and stacked area cards below
70 columns. Every tier SHALL preserve the same exact report facts.

#### Scenario: Wide terminal renders an area table
- **WHEN** the resolved width is 120 columns
- **THEN** area rows use aligned full columns and twelve-cell bars

#### Scenario: Medium terminal renders a compact table
- **WHEN** the resolved width is 80 columns
- **THEN** area rows use aligned compact columns and eight-cell bars

#### Scenario: Narrow terminal renders stacked cards
- **WHEN** the resolved width is 50 columns
- **THEN** each area uses a stacked card and a ten-cell bar without breaking source locations or drill commands

### Requirement: Terminal styling is optional and semantic
Terminal styling SHALL use ANSI sequences only when the CLI resolves color as
enabled. Styling SHALL distinguish High or Worse, Watch or Changed, Better,
coverage gaps, navigation, headings, and secondary text without backgrounds.
Symbols and Unicode bars SHALL remain in plain redirected output.

#### Scenario: Styled and plain output are compared
- **WHEN** the same report, width, and detail choice are rendered with color on and off
- **THEN** removing ANSI sequences from styled output produces the plain output byte for byte

#### Scenario: JSON is requested
- **WHEN** a user selects JSON output
- **THEN** the JSON contains no ANSI styling and remains schema version 1

### Requirement: Terminal counts and labels are easy to scan
Terminal rendering SHALL group large integer digits, use correct singular or
plural wording, display table zeroes as an en dash, and align using Unicode
display width. Only area labels may be shortened in the middle.

#### Scenario: Area name exceeds its column
- **WHEN** a wide or compact table cannot fit an area label
- **THEN** the label is shortened in the middle while its ending and all other columns remain visible

#### Scenario: Location exceeds terminal width
- **WHEN** a finding location is wider than the resolved terminal
- **THEN** the full location remains present and copyable

### Requirement: Color and width are CLI-owned choices
The CLI SHALL accept `--color auto|always|never`. Automatic mode SHALL enable
color only for a terminal when `NO_COLOR` is absent. Explicit color selection
SHALL conflict with `--json`. `COLUMNS` SHALL override width; otherwise a
connected terminal SHALL use its reported width and a redirected stream SHALL
use 100 columns.

#### Scenario: Output is redirected
- **WHEN** standard output is not a terminal and no width or color override is supplied
- **THEN** the terminal renderer receives width 100 and color disabled while retaining Unicode layout

#### Scenario: User forces color through a pipe
- **WHEN** standard output is redirected with `--color always`
- **THEN** ANSI styling is present

#### Scenario: User disables automatic color
- **WHEN** `NO_COLOR` is present or `--color never` is supplied
- **THEN** ANSI styling is absent

### Requirement: Package summary follows package scopes
Repository summaries SHALL count descendant package scopes. A selected package,
directory, or file SHALL report its nearest package ancestor once and SHALL
report zero when it has none.

#### Scenario: Repository has nested source scopes
- **WHEN** one package contains many directories and files
- **THEN** the repository summary counts that package once

#### Scenario: File scope is selected
- **WHEN** a selected file belongs to a package
- **THEN** its summary reports one package rather than counting nested scopes

### Requirement: Diff details state only meaningful changes
Terminal comparison cards SHALL state direction and identity, retain an
unbroken location, and show only measurements whose values changed. Added,
removed, and unsafe-to-match units SHALL use direct explanatory text.

#### Scenario: One measurement changes
- **WHEN** a retained comparison changes only cognitive complexity
- **THEN** its terminal card shows the cognitive before and after values without repeating unchanged measurements

#### Scenario: Unit cannot be matched safely
- **WHEN** a retained comparison is ambiguous
- **THEN** its terminal card says that the identity could not be matched safely
