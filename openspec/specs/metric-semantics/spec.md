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
and parse status. Existing output SHALL NOT override these expected values when
it conflicts with the defined metric rules.

#### Scenario: New engine corrects an existing value
- **WHEN** an exact fixture proves that the current analyzer counted syntax with the wrong meaning
- **THEN** the expected value changes with a documented reason and all affected product snapshots are reviewed

#### Scenario: Grammar dependency changes
- **WHEN** a grammar update changes a node shape used by a language implementation
- **THEN** exact language fixtures fail until the semantic translation is reviewed
