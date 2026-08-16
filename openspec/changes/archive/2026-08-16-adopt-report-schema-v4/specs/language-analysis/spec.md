## MODIFIED Requirements

### Requirement: Units expose only rated measurements
Each unit SHALL expose its name, container identity, kind, original source span, cognitive
complexity, cyclomatic complexity, exclusive logical lines, maximum nesting
depth, and parameter count. All five measurements SHALL be rated measurements
from this change onward, so a unit's rating SHALL be explainable entirely from
the five serialized measurements. Smackdebt SHALL NOT carry upstream Halstead or
maintainability structures into analysis policy or reports.

#### Scenario: Health policy rates a unit
- **WHEN** a language implementation produces a unit
- **THEN** health policy can explain its rating entirely from the five rated measurements

#### Scenario: A unit is rated for nesting alone
- **WHEN** a unit's cognitive, cyclomatic, and statement values are healthy and its maximum nesting depth is 7
- **THEN** its rating is High and the serialized measurements explain why
