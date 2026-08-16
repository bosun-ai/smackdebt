# metric-semantics Specification

## Purpose
Define shared source measurements and the language translation contract that
feeds them.
## Requirements
### Requirement: Language syntax is translated once
Each supported language SHALL implement the private language contract that
translates its grammar nodes into unit boundaries, control-flow events, logical
statements, injection regions, names, containers, and source spans. A node
SHALL be classified at most once during one unit traversal.

#### Scenario: Two languages express a condition differently
- **WHEN** Rust and Ruby grammar nodes represent equivalent conditional control flow
- **THEN** their language implementations emit the same conditional meaning and the shared algorithms do not inspect either grammar

#### Scenario: Generic algorithm receives a syntax node
- **WHEN** a measurement algorithm is compiled
- **THEN** its interface accepts only shared semantic values and contains no concrete language identifier or grammar node name

### Requirement: Cognitive complexity has one defined algorithm
Cognitive complexity SHALL start at zero for each rated unit and SHALL add
language-provided structural, nesting, alternative, labeled-jump, and
boolean-run costs through one shared algorithm. Nested rated units SHALL NOT
contribute to their parent's cognitive value. Recursion SHALL NOT contribute
without a separate reliable call graph.

#### Scenario: Equivalent nested control flow appears in two languages
- **WHEN** two language implementations emit the same ordered cognitive events
- **THEN** the shared algorithm returns the same cognitive value

#### Scenario: Function contains a closure
- **WHEN** the closure is a separately rated unit
- **THEN** the parent and closure receive independent cognitive measurements without counting the closure body twice

### Requirement: Cyclomatic complexity has one defined algorithm
Cyclomatic complexity SHALL start at one for each rated unit and SHALL add one
for every language-provided independent decision event. Nested rated units
SHALL NOT contribute decisions to their parent.

#### Scenario: Unit contains conditions and boolean decisions
- **WHEN** its language implementation emits three independent decision events
- **THEN** the unit's cyclomatic complexity is four

#### Scenario: Similar syntax does not create another path
- **WHEN** a language implementation classifies a construct as non-decision syntax
- **THEN** the shared algorithm does not increase cyclomatic complexity

### Requirement: Logical lines count statements rather than physical layout
Exclusive logical lines SHALL count language-provided executable or declarative
statements within one rated unit. Blank lines, comments, wrapper declarations,
markup, and separately rated nested units SHALL NOT count.

#### Scenario: Several statements share one physical line
- **WHEN** the language implementation emits three logical statements on that line
- **THEN** the unit records three logical lines

#### Scenario: One statement spans several physical lines
- **WHEN** the language implementation emits one logical statement for that syntax
- **THEN** the unit records one logical line

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

### Requirement: Maximum nesting depth has one defined algorithm
Maximum nesting depth SHALL be the deepest nesting level reached inside one rated unit. It
SHALL start at zero at the unit body and SHALL increase through the same
language-provided nesting events that cognitive complexity already uses, so no
language emits a separate nesting vocabulary. Separately rated nested units
SHALL NOT contribute depth to their parent.

Maximum nesting depth SHALL be a rated signal: Watch at 4 and High at 7. A value
equal to a threshold SHALL trigger that threshold. Both thresholds SHALL be
configurable and SHALL require no configuration to exist.

#### Scenario: A unit nests three levels
- **WHEN** a language implementation emits three ordered nesting entries without leaving them
- **THEN** the unit's maximum nesting depth is three and no nesting signal triggers

#### Scenario: A unit reaches the Watch threshold
- **WHEN** a unit's maximum nesting depth is 4
- **THEN** the nesting signal triggers Watch

#### Scenario: A unit reaches the High threshold
- **WHEN** a unit's maximum nesting depth is 7
- **THEN** the nesting signal triggers High

#### Scenario: A unit contains a rated closure
- **WHEN** the closure is a separately rated unit and nests deeply inside itself
- **THEN** the closure reports its own depth and the parent's depth excludes the closure body

### Requirement: Parameter count has one defined definition
Parameter count SHALL be the number of parameters declared by a rated unit, counted once
per declared parameter. A receiver SHALL be counted only when the language
declares it explicitly as a parameter. A rated unit that cannot declare
parameters SHALL report zero. The shared contract SHALL provide a default so a
language implementation supplies the value only where it is meaningful.

Parameter count SHALL be a rated signal: Watch at 6 and High at 9. A value equal
to a threshold SHALL trigger that threshold. Both thresholds SHALL be
configurable and SHALL require no configuration to exist.

#### Scenario: A function declares five parameters
- **WHEN** a rated unit declares five parameters
- **THEN** its parameter count is five and no parameter signal triggers

#### Scenario: A function reaches the Watch threshold
- **WHEN** a rated unit declares six parameters
- **THEN** the parameter signal triggers Watch

#### Scenario: A function reaches the High threshold
- **WHEN** a rated unit declares nine parameters
- **THEN** the parameter signal triggers High

#### Scenario: A unit cannot declare parameters
- **WHEN** a rated unit is a template unit or another form without a parameter list
- **THEN** its parameter count is zero and no parameter signal triggers

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
