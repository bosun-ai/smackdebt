## ADDED Requirements

### Requirement: Product documentation explains evolutionary signals

The README SHALL explain churn, package change coupling, contributor count,
contributor concentration, and unexplained-coupling Watch findings in plain
product language.

#### Scenario: A user interprets evolution output

- **WHEN** the user reads the README and an example report
- **THEN** the user can distinguish present code health, static architecture,
  and change-history evidence
- **AND** the user is not told that any descriptive value is automatically bad

### Requirement: Product documentation states privacy and coverage behavior

The README SHALL state that analysis stays local, contributor identities are
not reported, and unavailable or incomplete history is shown explicitly.

#### Scenario: A user analyzes a shallow repository

- **WHEN** the user consults history documentation
- **THEN** the documented output matches the incomplete-history diagnostic
- **AND** it explains which source and architecture results still remain usable

