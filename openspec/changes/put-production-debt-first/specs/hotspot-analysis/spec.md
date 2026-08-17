## MODIFIED Requirements

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
