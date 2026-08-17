# hotspot-analysis Specification

## Purpose
TBD - created by archiving change deepen-debt-signals. Update Purpose after archive.
## Requirements
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
Finding rank SHALL order by rating, role class, hot before not hot, signals at
that rating, total triggered signals, cognitive complexity, cyclomatic
complexity, logical lines, activity, path, and span, in that order. Role class
and hot state SHALL decide before either signal count. This supersedes the
earlier order, which placed signals at that rating and total triggered signals
above role class and hot state; because those counts rarely tie over a real
repository, role and heat almost never decided and the headline named test or
benchmark code. Non-primary findings SHALL remain visible below primary findings
at equal rating rather than being removed, and a selection whose only findings
are non-primary SHALL still name a worst offender rather than naming none. No
role filter SHALL be applied to worst-offender selection or to the finding list;
the rank alone SHALL produce the preference. The order SHALL remain total and
data-stable so serial and parallel runs produce identical output.

#### Scenario: Cold production debt meets hot test debt
- **WHEN** a primary finding that is not hot and a test finding that is hot have the same rating
- **THEN** the primary finding ranks first and the test finding remains present below it

#### Scenario: Hot debt meets equally rated cold debt with more signals
- **WHEN** two primary findings share a rating and the one belonging to a hotspot file has fewer signals at that rating and fewer total triggered signals
- **THEN** the hot finding ranks first, because hot state decides before either signal count

#### Scenario: Every finding in a selection is non-primary
- **WHEN** a selection's only rated findings carry a non-primary role
- **THEN** the highest ranked of them is named as the worst offender rather than no worst offender being named

#### Scenario: Rating, role class, and hot state all tie
- **WHEN** two findings tie on rating, role class, and hot state
- **THEN** ordering continues with signals at that rating, total triggered signals, cognitive complexity, cyclomatic complexity, logical lines, activity, path, and span in that order

#### Scenario: Serial and parallel runs rank the same findings
- **WHEN** the same selection is analyzed serially and in parallel
- **THEN** the ranked finding order is identical, because every key is a data value rather than a discovery order
