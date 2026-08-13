## ADDED Requirements

### Requirement: Reports identify an initial selected scope
Every completed codebase and diff report SHALL identify one initial selected
scope separately from the report facts. A renderer SHALL be able to render any
scope retained by that report without changing the report or running project
work again.

#### Scenario: Root report is explored several times
- **WHEN** a renderer selects the repository, a package, and a directory from one completed root report
- **THEN** every view is produced from retained report facts without filesystem, Git, parser, or worker activity

#### Scenario: Explicit path limits the report
- **WHEN** a user runs Smackdebt with an existing directory path
- **THEN** that directory is the initial selected scope and its shares use that scope's totals

#### Scenario: Selected directory has no supported source
- **WHEN** an existing selected directory contains no supported source files
- **THEN** Smackdebt produces a successful zero-total selected scope instead of replacing it with another scope

### Requirement: Codebase scopes expose debt distribution
Each non-file codebase scope SHALL expose exact High, Watch, and Healthy counts
for its children. A child's debt share SHALL equal its High plus Watch count
divided by the selected scope's High plus Watch count.

#### Scenario: Debt is spread across child directories
- **WHEN** the selected scope contains several child areas with rated units
- **THEN** each row shows exact health counts and a nearest-whole-percent share derived from those counts

#### Scenario: Selected scope has no debt
- **WHEN** the selected scope has zero High and zero Watch units
- **THEN** every child debt share is `0%` and no combined health score is shown

#### Scenario: Rounded shares do not sum to one hundred
- **WHEN** independently rounded child shares do not total 100 percent
- **THEN** the exact High and Watch counts remain the authoritative values

### Requirement: Codebase child order is severity-led
Codebase child rows SHALL sort by High count descending, Watch count descending,
then repository-relative path ascending. Debt-bearing children SHALL precede
healthy-only children when the default row limit is applied.

#### Scenario: Children have different health distributions
- **WHEN** one child has more High units and another has more total debt units
- **THEN** the child with more High units appears first

#### Scenario: Healthy children fit in the default view
- **WHEN** fewer than ten debt-bearing children exist
- **THEN** healthy-only children fill the remaining default rows in path order

### Requirement: Structural single-child chains are passed visibly
Terminal rendering SHALL pass through repository, package, or directory scopes
that have exactly one child until it reaches a file or a scope with several
children. It SHALL show the passed repository-relative path as a breadcrumb.

#### Scenario: Repository contains one root package and one source directory
- **WHEN** the root package and source directory each have one child
- **THEN** the terminal view passes through both, prints the resulting breadcrumb, and avoids an empty `.` package row

#### Scenario: JSON consumer reads the same report
- **WHEN** terminal rendering passes through structural scopes
- **THEN** JSON still contains every original scope and parent-child link

### Requirement: Default terminal detail stays concise
The default terminal view SHALL show at most ten child rows and the first three
retained findings or comparisons within the selected scope. It SHALL report the
number of omitted debt-bearing and healthy-only child rows separately.

#### Scenario: Selected scope has more than ten children
- **WHEN** the default terminal report omits child rows
- **THEN** it states exact omitted counts and does not imply that the visible rows are complete

#### Scenario: User requests complete terminal detail
- **WHEN** the user supplies `--all`
- **THEN** terminal output shows every child row and every retained finding or comparison in the selected scope

#### Scenario: User requests complete JSON
- **WHEN** the user supplies `--json`
- **THEN** JSON contains every retained row and detail without requiring `--all`

#### Scenario: User combines incompatible output options
- **WHEN** the user supplies `--all --json`
- **THEN** argument parsing rejects the invocation with exit code 2 and a simple explanation

### Requirement: Codebase detail follows existing hotspot priority
Non-file codebase views SHALL order retained findings by health, activity,
measurements, path, and source span using the existing visible hotspot rules.
File views SHALL show every retained finding, grouped by container when present,
with a repository-relative `path:line` location.

#### Scenario: Selected directory contains many findings
- **WHEN** the default terminal view renders the directory
- **THEN** it shows its first three findings and does not select findings from outside that directory

#### Scenario: User drills into a file
- **WHEN** a selected file contains several retained findings
- **THEN** every finding is shown with its rating, measurements, and copyable source location

### Requirement: Explore points to the next debt-bearing area
The terminal `Explore` command SHALL target the first displayed child with at
least one High or Watch unit, using its repository-relative path.

#### Scenario: A deeper debt-bearing child exists
- **WHEN** the selected codebase scope has a debt-bearing child
- **THEN** the report prints one valid `smackdebt <path>` command for that child

#### Scenario: No deeper debt-bearing child exists
- **WHEN** the selected scope is a file or all deeper children are healthy
- **THEN** the report omits the `Explore` section

### Requirement: Diff comparisons have one displayed direction
Every retained comparison SHALL belong to exactly one of Worse, Better, or
Changed. `Regressed` and debt-bearing `Added` comparisons SHALL be Worse.
`Improved` and debt-bearing `Removed` comparisons SHALL be Better. All other
retained comparisons SHALL be Changed.

#### Scenario: Watch unit is added
- **WHEN** a comparison is `Added` with an after rating of Watch
- **THEN** it contributes once to Worse for its file and every ancestor scope

#### Scenario: High unit is removed
- **WHEN** a comparison is `Removed` with a before rating of High
- **THEN** it contributes once to Better for its file and every ancestor scope

#### Scenario: Healthy unit is added or removed
- **WHEN** an added or removed unit is Healthy on its existing side
- **THEN** it contributes once to Changed

#### Scenario: Measurements move within one rating
- **WHEN** a comparison is `MetricChanged`
- **THEN** it contributes once to Changed while retaining its detailed kind and measurements

### Requirement: Diff scopes expose change distribution
Each diff scope SHALL expose exact Worse, Better, and Changed counts. A child's
share SHALL equal its sum of those counts divided by the selected scope's sum.

#### Scenario: Changed units span several directories
- **WHEN** the selected diff scope has changes in several child areas
- **THEN** each row shows exact direction counts and its nearest-whole-percent share of selected changed units

#### Scenario: Diff selection has no retained comparisons
- **WHEN** the selected scope has a zero change-count denominator
- **THEN** every child share is `0%`

### Requirement: Diff child and detail order is deterministic
Diff child rows SHALL sort by Worse descending, Better descending, Changed
descending, then repository-relative path ascending. Comparison detail SHALL
group Worse before Better before Changed and use stable path and source order
within each direction.

#### Scenario: Several areas changed
- **WHEN** one child has more Worse outcomes than another
- **THEN** it appears first even when the other child has more total changes

#### Scenario: File diff is selected
- **WHEN** the selected file has several detailed comparison kinds
- **THEN** the terminal view shows every comparison in stable direction, path, and source order

### Requirement: JSON version 1 exposes progressive links additively
JSON schema version 1 SHALL add the initial selected scope, indexed report paths,
scope finding and comparison links, scope diff counts, comparison file ownership,
and comparison direction. Existing fields, types, and meanings SHALL remain
unchanged.

#### Scenario: Integration reads a codebase report
- **WHEN** JSON schema version 1 contains progressive fields
- **THEN** the integration can traverse from the selected scope to children and retained findings using indexes

#### Scenario: Integration reads a diff report
- **WHEN** JSON schema version 1 contains comparisons
- **THEN** each comparison identifies its file, detailed kind, derived direction, measurements, and rating transition

#### Scenario: Terminal output is truncated
- **WHEN** terminal row or detail limits apply
- **THEN** JSON still serializes the complete retained report
