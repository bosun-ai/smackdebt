## MODIFIED Requirements

### Requirement: Architecture integrates with progressive reports
Default terminal reports SHALL show architecture health totals and rated cycle
witnesses without arbitrary dependency-edge rows. `--all` and path drill SHALL
show relevant incoming, outgoing, unresolved, ambiguous, advisory, and
module-ownership relations. The machine report SHALL retain complete relation
tables in whichever schema version is current, so retiring one version never
retires the relation tables.

#### Scenario: A repository has many ordinary uses and no cycles
- **WHEN** default terminal output is rendered
- **THEN** architecture states its health without listing arbitrary edges

#### Scenario: A user drills into one package
- **WHEN** incoming, outgoing, ownership, or advisory relations touch it
- **THEN** detailed output shows those relations without unrelated graph rows

#### Scenario: A machine consumer reads relations
- **WHEN** the current machine report is parsed
- **THEN** every dependency edge, package edge, external dependency, resolution diagnostic, and package-graph row remains present
