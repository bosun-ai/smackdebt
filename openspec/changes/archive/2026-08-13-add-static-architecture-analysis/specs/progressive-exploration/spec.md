## ADDED Requirements

### Requirement: Codebase reports separate code and architecture health
The default codebase report SHALL retain its code quality section and SHALL add
an architecture section with dependency coverage, High and Watch architecture
finding counts, leading affected areas, descriptive graph measurements, and
concise cycle witnesses. It SHALL NOT combine code and architecture into one
score.

#### Scenario: Repository has code and architecture findings
- **WHEN** one invocation analyzes both result sets
- **THEN** each section presents its own counts, reasons, and next drill target

### Requirement: Diff reports separate source and architecture change
The diff report SHALL retain source comparisons and SHALL add architecture
Worse, Better, and Changed counts, affected areas, edge changes, and cycle
witnesses.

#### Scenario: Worktree improves code and introduces a cycle
- **WHEN** source complexity decreases while a dependency edge closes a package cycle
- **THEN** source output reports the improvement and architecture output reports the Worse cycle without offsetting either result

### Requirement: Architecture detail follows selected scope
Architecture presentation SHALL include findings and edges whose source or
target belongs to the selected scope. Incoming facts from outside the selection
SHALL remain visible when they explain the selection. Unrelated graph regions
SHALL not appear in the selected terminal view.

#### Scenario: Package is selected
- **WHEN** another package depends on it
- **THEN** the incoming relationship and its source package are visible in architecture detail

### Requirement: Terminal limits do not limit architecture JSON
Default architecture presentation SHALL stay concise and `--all` SHALL show all
retained architecture rows and findings in the selected scope. JSON version 2
SHALL contain the complete static graph and every retained architecture fact
regardless of terminal limits.

#### Scenario: Repository has many dependency edges
- **WHEN** default terminal output omits edge detail
- **THEN** JSON and `--all` retain the complete applicable architecture information
