## MODIFIED Requirements

### Requirement: README demonstrates debt distribution
The README SHALL show the responsive codebase dashboard with exact child High,
Watch, debt-share, and local-rate values, rate bars, leading findings, and the
next drill command.

#### Scenario: New user follows progressive exploration
- **WHEN** the user reads the codebase example
- **THEN** the example explains the quality summary and moves from repository to child area to file using exact, internally consistent counts

### Requirement: README demonstrates progressive diff exploration
The README SHALL show a responsive diff dashboard with child Worse, Better,
Changed, and share values, share bars, and meaningful unit changes.

#### Scenario: User locates a worktree regression
- **WHEN** the user reads the diff example
- **THEN** the example shows how to identify the affected area and drill to its concise detailed comparison

## ADDED Requirements

### Requirement: README documents terminal presentation controls
Product documentation SHALL explain `--color`, `NO_COLOR`, width behavior,
Unicode redirected output, `--all`, and JSON as the complete machine-readable
view.

#### Scenario: User runs Smackdebt in automation
- **WHEN** the user reads output guidance
- **THEN** the README explains how to force or disable ANSI styling and that JSON remains unstyled
