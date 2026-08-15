## ADDED Requirements

### Requirement: README presents JSON version 4 and exact examples
The README SHALL identify version 4 as the machine contract, link its checked schema, and
describe the denormalized head — `verdict` with tier, sentence, and mode, and
`summary` with counts and up to three fully resolved worst entries — as the way
to answer the common question without joining tables. It SHALL state that every
serialized value is an integer or a string, that similarity and concentration
are published as their integer operands rather than as ratios, and that no
documented text promises a serialized similarity or ratio value. Its executable
examples SHALL assert exact status, stderr, and all declared stdout fragments in
order.

#### Scenario: A documented example changes
- **WHEN** its result no longer matches the declared behavior
- **THEN** documentation acceptance fails with a reviewable difference

#### Scenario: A user looks for coupling ratios in JSON
- **WHEN** they read the JSON section
- **THEN** it states that shared and union commit counts are published and that any ratio is derived by the consumer

## MODIFIED Requirements

### Requirement: README states JSON detail retention
The README SHALL state that JSON retains every Watch and High finding and all scope
summaries, while healthy units are represented through aggregate counts. It
SHALL also state that JSON is the complete view of every fact the terminal
omits, including raw dependency edges, external references, churn detail, weak
coupling, hotspots, size findings, and orphan files.

#### Scenario: Integration author chooses JSON output
- **WHEN** an integration needs all debt findings
- **THEN** the README makes clear that JSON is complete for Watch and High findings but not a full healthy-unit index

#### Scenario: A user misses a row the terminal removed
- **WHEN** they look for a row that human output no longer prints
- **THEN** the README directs them to the version-4 JSON table that retains it

## REMOVED Requirements

### Requirement: README presents JSON version 3 and exact examples
**Reason**: Version 3 is retired by this change.
**Migration**: Replaced by `README presents JSON version 4 and exact examples`.
