## ADDED Requirements

### Requirement: README explains integrated architecture analysis
The README SHALL show that default and diff commands report code and
architecture in separate sections from one invocation. It SHALL explain cycles,
fan-in, fan-out, instability, dependency coverage, architecture change
directions, and progressive drill-down without presenting one combined score.

#### Scenario: New user evaluates a repository
- **WHEN** the README example contains a code finding and a package cycle
- **THEN** the example explains both results independently and shows useful next commands

### Requirement: README explains static-analysis limits
The README SHALL distinguish resolved internal, external, unresolved, and
ambiguous dependencies and SHALL state that Smackdebt does not provide compiler
type resolution, runtime tracing, or executed build configuration.

#### Scenario: User sees incomplete dependency coverage
- **WHEN** architecture output reports unresolved references
- **THEN** documentation explains why they are visible and why no edge was guessed

### Requirement: README documents version-2 architecture facts
The README SHALL describe complete static architecture tables in JSON schema
version 2 and SHALL state that terminal limits do not remove graph facts from
JSON.

#### Scenario: Integration needs every edge
- **WHEN** an author reads the JSON documentation
- **THEN** it identifies the file-edge, package-edge, graph-measurement, finding, comparison, and diagnostic relationships needed to traverse the report
