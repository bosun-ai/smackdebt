## ADDED Requirements

### Requirement: Hotspots cross rated debt with change activity
Analysis SHALL derive a hotspot for a file when the file has at least one rated
unit and its change activity within the analyzed history window reaches the
minimum touch count. The minimum touch count SHALL default to 5 and SHALL be
configurable without requiring configuration to exist. A hotspot SHALL retain
the file identity, the file's maximum unit rating, and its exact touch count as
integer operands, and SHALL NOT introduce a combined score, a floating-point
value, or an invented weight. Files without a rated unit and files below the
minimum touch count SHALL NOT be hotspots. Hotspots SHALL be derived from tables
the report already builds, without a new file read, traversal, or Git process,
and SHALL be ordered by data-stable keys.

#### Scenario: A rated file changes often
- **WHEN** a file has a High rated unit and 14 touches in the window with a minimum touch count of 5
- **THEN** a hotspot exists for that file retaining rating High and touch count 14

#### Scenario: A file is at the touch boundary
- **WHEN** one file has 4 touches and another has 5 touches, both with rated units and a minimum of 5
- **THEN** only the file with 5 touches is a hotspot

#### Scenario: An unrated file changes constantly
- **WHEN** a file has no rated unit and 40 touches
- **THEN** no hotspot exists for that file

### Requirement: Finding rank prefers hot production debt
Finding rank SHALL order by rating, signals at that rating, total triggered
signals, hot before not hot, primary role before non-primary role, cognitive
complexity, cyclomatic complexity, logical lines, activity, path, and span, in
that order. Non-primary findings SHALL remain visible below primary findings at
equal rating rather than being removed. The order SHALL remain total and
data-stable so serial and parallel runs produce identical output.

#### Scenario: Production and test debt tie on rating
- **WHEN** a primary finding and a test finding have the same rating, signals, and triggered-signal count and neither is hot
- **THEN** the primary finding ranks first and the test finding remains present below it

#### Scenario: Hot debt meets equally rated cold debt
- **WHEN** two findings tie through total triggered signals and only one belongs to a hotspot file
- **THEN** the hot finding ranks first

#### Scenario: Both new keys tie
- **WHEN** two findings tie on rating, signals, triggered signals, hot state, and role class
- **THEN** ordering continues with cognitive complexity, cyclomatic complexity, logical lines, activity, path, and span exactly as before
