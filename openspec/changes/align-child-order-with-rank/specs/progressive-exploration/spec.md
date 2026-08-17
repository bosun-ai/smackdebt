## MODIFIED Requirements

### Requirement: Codebase child order is severity-led
Codebase child area rows SHALL sort by High count descending, Watch count
descending, then repository-relative name ascending, so the child holding the
worst debt is read first and equal children keep a stable name order. Codebase
finding rows SHALL use the finding rank owned by `hotspot-analysis` rather than
any order of their own, and this specification SHALL NOT restate that rank's
keys. This supersedes the earlier text, which enumerated a finding rank of
rating, signals at that rating, total triggered signals, metrics, and recent
activity; that copy omitted role class and hot state, contradicted the accepted
rank once role and heat moved ahead of both signal counts, and described neither
the child rows nor the finding rows as implemented.

#### Scenario: Two child areas hold different debt
- **WHEN** one displayed child area has more High units than another
- **THEN** it appears first even when the other has more Watch units

#### Scenario: Two child areas hold equal debt
- **WHEN** two displayed child areas have equal High and Watch counts
- **THEN** their repository-relative names produce stable order

#### Scenario: Hot production debt meets colder debt in one scope
- **WHEN** the selected scope holds findings that differ in role class or hot state
- **THEN** the displayed finding order is exactly the finding rank `hotspot-analysis` accepts, so primary source precedes non-primary source at equal rating and hot state decides before either signal count

#### Scenario: Every rank key ties
- **WHEN** two findings tie on every key the finding rank compares before path
- **THEN** path and span produce stable order
