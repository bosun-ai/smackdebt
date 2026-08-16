## ADDED Requirements

### Requirement: Maximum nesting depth has one defined algorithm
Maximum nesting depth SHALL be the deepest nesting level reached inside one
rated unit. It SHALL start at zero at the unit body and SHALL increase through
the same language-provided nesting events that cognitive complexity already
uses, so no language emits a separate nesting vocabulary. Separately rated
nested units SHALL NOT contribute depth to their parent. The value SHALL be
collected and exposed as a measurement in this change and SHALL NOT be rated
until `adopt-report-schema-v4` promotes it.

#### Scenario: A unit nests three levels
- **WHEN** a language implementation emits three ordered nesting entries without leaving them
- **THEN** the unit's maximum nesting depth is three

#### Scenario: A unit contains a rated closure
- **WHEN** the closure is a separately rated unit and nests deeply inside itself
- **THEN** the closure reports its own depth and the parent's depth excludes the closure body

#### Scenario: A flat unit is measured
- **WHEN** a unit contains no nesting events
- **THEN** its maximum nesting depth is zero

### Requirement: Parameter count has one defined definition
Parameter count SHALL be the number of parameters declared by a rated unit,
counted once per declared parameter. A receiver SHALL be counted only when the
language declares it explicitly as a parameter. A rated unit that cannot declare
parameters SHALL report zero. The shared contract SHALL provide a default so a
language implementation supplies the value only where it is meaningful. The
value SHALL be collected and exposed as a measurement in this change and SHALL
NOT be rated until `adopt-report-schema-v4` promotes it.

#### Scenario: A function declares parameters
- **WHEN** a rated unit declares four parameters
- **THEN** its parameter count is four

#### Scenario: A unit cannot declare parameters
- **WHEN** a rated unit is a template unit or another form without a parameter list
- **THEN** its parameter count is zero

### Requirement: File and container size are rated signals
File size SHALL use the file's line count and SHALL be Watch at 400 lines and
High at 800 lines. Container size SHALL use the container's exclusive statement
total and SHALL be Watch at 300 statements and High at 600 statements. Both
thresholds SHALL be configurable and SHALL require no configuration to exist. A
value equal to a threshold SHALL trigger that threshold. Both signals SHALL be
explainable entirely from their serialized operands.

#### Scenario: A file reaches the Watch threshold
- **WHEN** a file has exactly 400 lines
- **THEN** it produces a Watch size finding retaining its exact line count

#### Scenario: A container exceeds the High threshold
- **WHEN** a container's exclusive statement total is 640
- **THEN** it produces a High size finding retaining that exact total

#### Scenario: A small file is measured
- **WHEN** a file has 120 lines
- **THEN** no size finding is created

## MODIFIED Requirements

### Requirement: Exact fixtures define metric compatibility
Each supported language SHALL have readable fixtures with exact units,
identities, spans, cognitive values, cyclomatic values, logical-line values,
maximum nesting values, parameter counts, and parse status. Existing output
SHALL NOT override these expected values when it conflicts with the defined
metric rules.

#### Scenario: New engine corrects an existing value
- **WHEN** an exact fixture proves that the current analyzer counted syntax with the wrong meaning
- **THEN** the expected value changes with a documented reason and all affected product snapshots are reviewed

#### Scenario: Grammar dependency changes
- **WHEN** a grammar update changes a node shape used by a language implementation
- **THEN** exact language fixtures fail until the semantic translation is reviewed

#### Scenario: A new measurement is added
- **WHEN** maximum nesting depth and parameter count are collected
- **THEN** every supported language has exact fixture values for both before the measurements are used
