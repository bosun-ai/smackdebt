## MODIFIED Requirements

### Requirement: Units expose only rated measurements
Each unit SHALL expose its name, container identity, kind, original source span,
cognitive complexity, cyclomatic complexity, exclusive logical lines, maximum
nesting depth, and parameter count. Cognitive complexity, cyclomatic complexity,
and exclusive logical lines SHALL remain the rated measurements in this change;
maximum nesting depth and parameter count SHALL be collected and exposed without
being rated. Smackdebt SHALL NOT carry upstream Halstead or maintainability
structures into analysis policy or reports.

#### Scenario: Health policy rates a unit
- **WHEN** a language implementation produces a unit
- **THEN** health policy can explain its rating entirely from the three rated measurements

#### Scenario: A collected measurement is inspected
- **WHEN** a unit exposes maximum nesting depth and parameter count
- **THEN** both values are available to analysis and neither changes the unit's rating in this change

## ADDED Requirements

### Requirement: Every language provides nesting and parameter fixtures
Every supported language SHALL have exact fixtures for maximum nesting depth and
parameter count before those measurements are consumed. Vue fixtures SHALL cover
script, script-setup, and template regions. A language whose units cannot
declare parameters SHALL have a fixture proving the value is zero.

#### Scenario: A supported language is verified
- **WHEN** its fixture suite runs
- **THEN** exact maximum nesting and parameter values are asserted for representative nested and parameterized units

#### Scenario: A Vue component is verified
- **WHEN** its fixtures run
- **THEN** script, script-setup, and template regions each assert exact nesting and parameter values
