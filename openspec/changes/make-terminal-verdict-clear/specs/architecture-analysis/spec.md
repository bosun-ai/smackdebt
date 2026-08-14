## MODIFIED Requirements

### Requirement: Static dependency coverage is visible
Every extracted reference SHALL contribute to resolved-internal, external,
unresolved, or ambiguous coverage. Report and JSON facts SHALL retain exact
category counts and each unresolved or ambiguous source location and reason.
External references SHALL not participate in the internal graph. Human terminal
output SHALL keep architecture resolution separate from source coverage and
SHALL group unmatched and ambiguous imports into exactly one row: `<Warning
glyph> 1 import could not be followed.` or `<Warning glyph> <count> imports
could not be followed.` Human output SHALL NOT expose separate category totals.

#### Scenario: Repository uses external packages
- **WHEN** a reference safely identifies an external package but no internal file
- **THEN** the package receives an external dependency count without an internal graph edge

#### Scenario: Some references cannot be followed
- **WHEN** a codebase report retains unmatched or ambiguous imports
- **THEN** human output shows one Warning-glyph row with exact singular or plural `could not be followed` wording, separate from source coverage
- **AND** report and JSON retain exact unresolved and ambiguous counts, locations, and reasons
