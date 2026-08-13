## ADDED Requirements

### Requirement: Architecture defines progressive report navigation
The architecture document SHALL describe report path ownership, selected-scope
ownership, codebase and diff hierarchy construction, finding and comparison
links, and the separation between exact aggregation and display policy.

#### Scenario: Engineer adds another renderer
- **WHEN** a renderer needs repository, package, directory, file, or code-unit detail
- **THEN** the architecture identifies how it selects and traverses retained facts without running analysis

### Requirement: Architecture defines distribution arithmetic
The architecture document SHALL define codebase debt share, diff change share,
three-way diff direction, severity-led ordering, zero denominators, integer
rounding, and smart single-child display behavior.

#### Scenario: Terminal and another renderer show one scope
- **WHEN** both renderers display distribution for the same selected scope
- **THEN** they derive counts and shares from the same documented report facts

### Requirement: Architecture defines selected-path work limits
The architecture document SHALL explain how an explicit path retains
repository-relative identity while limiting discovery, source reads, and share
denominators to the selected area.

#### Scenario: Engineer changes path selection
- **WHEN** selection behavior is updated
- **THEN** the architecture identifies the identity, package-ancestor, source-read, and performance rules that must remain true
