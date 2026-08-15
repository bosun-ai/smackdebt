## ADDED Requirements

### Requirement: Verdict policy has exact boundary evidence
Public generated repositories SHALL provide hand-calculated facts for every
codebase tier including both permille boundaries at exactly 1% and exactly 5%,
both architecture escalation floors, a floor that must not lower an already
higher tier, all four diff tiers, the contradiction case where no source
comparison moves debt and a package cycle is introduced, a diff whose only
changes are healthy additions, and worst-offender selection for the hotspot
reason, the complexity reason, the cycle fallback, and the absent case. Evidence
SHALL prove that scope verdicts differ from the root verdict where the facts
differ, that a duplicate debt-diff identity fails index integrity, that no
floating-point value participates in tier selection, and that serial and
parallel runs produce identical verdicts and selections.

#### Scenario: Tier fixtures run
- **WHEN** the real CLI analyzes the verdict fixtures
- **THEN** every tier id, sentence, count, and worst offender matches its hand-calculated value

#### Scenario: The contradiction case runs
- **WHEN** a diff introduces a package cycle without moving any source comparison
- **THEN** the diff verdict is `worse` and its facts name the architecture family

#### Scenario: Index integrity is audited
- **WHEN** a scope's debt-diff selection contains a duplicate identity
- **THEN** acceptance fails before exact-byte approval
