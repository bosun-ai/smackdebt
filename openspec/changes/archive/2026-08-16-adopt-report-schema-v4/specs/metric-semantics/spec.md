## MODIFIED Requirements

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
