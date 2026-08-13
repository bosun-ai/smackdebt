## MODIFIED Requirements

### Requirement: JSON version 2 exposes complete progressive source facts
JSON schema version 2 SHALL expose the initial selected scope, indexed report
paths, scope finding and comparison links, scope diff counts, comparison file
ownership, comparison direction, exact source measurements, health, activity,
coverage, and diagnostics through flat tables. Terminal filtering SHALL NOT
remove retained JSON facts.

#### Scenario: Integration reads a codebase report
- **WHEN** JSON version 2 contains source findings
- **THEN** the integration can traverse from the selected scope to children, files, and retained findings using indexes

#### Scenario: Integration reads a diff report
- **WHEN** JSON version 2 contains source comparisons
- **THEN** each comparison identifies its file, detailed kind, direction, measurements, and rating transition

#### Scenario: Terminal output is limited
- **WHEN** terminal row or detail limits apply
- **THEN** JSON still serializes the complete retained version-2 source report

### Requirement: JSON output is unstyled and versioned
`--json` SHALL emit schema version 2 without ANSI styling. Explicit terminal
color selection SHALL remain incompatible with JSON output.

#### Scenario: JSON is requested
- **WHEN** a user runs any report mode with `--json`
- **THEN** standard output contains unstyled schema-version-2 JSON and no terminal presentation text
