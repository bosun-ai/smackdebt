## ADDED Requirements

### Requirement: README demonstrates debt distribution
The README SHALL show a codebase report with child High, Watch, and debt-share
values, leading findings, a passed breadcrumb when relevant, and the next drill
command.

#### Scenario: New user follows progressive exploration
- **WHEN** the user reads the codebase example
- **THEN** the example moves from repository to child area to file using exact, internally consistent counts

### Requirement: README demonstrates progressive diff exploration
The README SHALL show a diff report with child Worse, Better, Changed, and share
values before detailed unit comparisons.

#### Scenario: User locates a worktree regression
- **WHEN** the user reads the diff example
- **THEN** the example shows how to identify the affected area and drill to its detailed comparison

### Requirement: README explains share and completeness
The README SHALL define codebase and diff share denominators, integer rounding,
path-limited scope, default terminal limits, `--all`, and complete JSON behavior.

#### Scenario: User interprets a percentage
- **WHEN** a displayed child share is rounded
- **THEN** the README directs the user to exact counts and explains why displayed shares may not sum to 100 percent

#### Scenario: User needs every retained result
- **WHEN** the default terminal view omits rows or details
- **THEN** the README explains terminal `--all` and that JSON is always complete
