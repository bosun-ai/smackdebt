## MODIFIED Requirements

### Requirement: Diff details state only meaningful changes
Terminal comparison cards SHALL state direction and identity, retain an
unbroken location, and show only measurements whose values changed. Added,
removed, and unsafe-to-match units SHALL keep their direct explanatory text, and
that text SHALL come first on the card. An added or removed unit has one side
only, so its card SHALL additionally state that present side's absolute
measurements — the after side for an added unit and the before side for a
removed unit — showing only measurements whose value is nonzero. When every
measurement of the present side is zero, the card SHALL be the direction word
alone. An unsafe-to-match card SHALL remain its explanatory sentence alone,
because no side of it can be trusted. These values SHALL be taken from the
comparison the report already carries, and the renderer SHALL derive nothing.

#### Scenario: One measurement changes
- **WHEN** a retained comparison changes only cognitive complexity
- **THEN** its terminal card shows the cognitive before and after values without repeating unchanged measurements

#### Scenario: A unit is added
- **WHEN** a comparison adds a unit whose after side has nonzero measurements
- **THEN** its terminal card states the direction word first and then the after-side absolute values of those nonzero measurements

#### Scenario: A unit is removed
- **WHEN** a comparison removes a unit whose before side has nonzero measurements
- **THEN** its terminal card states the direction word first and then the before-side absolute values of those nonzero measurements

#### Scenario: An added unit measures zero everywhere
- **WHEN** an added unit's after-side measurements are all zero
- **THEN** its terminal card is the direction word alone with no zero-valued facts

#### Scenario: Unit cannot be matched safely
- **WHEN** a retained comparison is ambiguous
- **THEN** its terminal card says that the identity could not be matched safely and states no measurement
