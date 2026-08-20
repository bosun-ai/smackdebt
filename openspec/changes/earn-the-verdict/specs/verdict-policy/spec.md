## MODIFIED Requirements

### Requirement: Tier mapping uses integer permille arithmetic
Tier selection SHALL evaluate, in order: zero checked units selects `empty`;
zero High and zero Watch selects `clean`; zero High selects `solid`; a High
permille of at most 10 selects `worn`; a High permille of at most 50 selects
`fights_back`; any greater value selects `lost`. High permille SHALL be the
integer division of High count multiplied by 1000 by the checked unit count. No
floating-point value SHALL be produced anywhere in tier selection. A value
exactly at a boundary SHALL select the lower tier.

Density mapping SHALL be tempered by two integer-only terms combined through
the existing max-floor rule beside the architecture floor:

- **Small-scope cap:** when fewer than 200 checked units exist and the High
  count is below 10, the density-mapped tier SHALL be capped at `worn`. The
  `empty`, `clean`, and `solid` selections are untouched, so the cap never
  lowers a scope below `worn` while High debt exists.
- **Volume floors:** a High count of at least 100 SHALL floor the tier at
  `fights_back`, and a High count of at least 1000 SHALL floor it at `lost`.
  Floors raise a density-mapped tier and never lower one.

The four constants — 200 checked units of density evidence, the 10-High
small-scope threshold, and the 100 and 1000 absolute High floors — are
proposed values under review and SHALL be implemented as named integer
constants so review can move them in one place.

#### Scenario: Nothing was checked
- **WHEN** the selected scope has zero checked units
- **THEN** the tier is `empty`

#### Scenario: No debt exists
- **WHEN** a scope has checked units with zero High and zero Watch findings
- **THEN** the tier is `clean`

#### Scenario: Only Watch findings exist
- **WHEN** a scope has Watch findings and zero High findings
- **THEN** the tier is `solid`

#### Scenario: High debt is exactly one percent
- **WHEN** a scope has 1000 checked units and 10 High findings
- **THEN** the High permille is 10 and the tier is `worn`

#### Scenario: High debt is exactly five percent
- **WHEN** a scope has 1000 checked units and 50 High findings
- **THEN** the High permille is 50 and the tier is `fights_back`

#### Scenario: High debt exceeds five percent
- **WHEN** a scope has 1000 checked units and 51 High findings
- **THEN** the tier is `lost`

#### Scenario: A tiny scope has one High finding
- **WHEN** a scope has 5 checked units and 1 High finding
- **THEN** the small-scope cap applies and the tier is `worn` rather than `lost`

#### Scenario: A tiny scope is saturated with High debt
- **WHEN** a scope has 20 checked units and 12 High findings
- **THEN** the cap does not apply because the High count reaches 10, and the tier is `lost`

#### Scenario: A huge scope dilutes many High findings
- **WHEN** a scope has 102000 checked units and 545 High findings
- **THEN** the volume floor raises the density-mapped `worn` to `fights_back`

#### Scenario: The absolute High count reaches one thousand
- **WHEN** a scope has 1000 or more High findings at any density
- **THEN** the tier is `lost`

## ADDED Requirements

### Requirement: The verdict qualifies unsupported coverage
Analysis SHALL compute the unsupported byte share of the selected scope as the
integer permille of unsupported source bytes over selected source bytes. When
that share exceeds 100 permille, the verdict SHALL carry the frozen qualifier
sentence `Not all source was checked.` together with a fact naming the share
and the largest unsupported language by bytes. The threshold constant is a
proposed value under review. The qualifier SHALL never change the selected
tier, SHALL be owned by analysis so every consumer prints identical bytes, and
SHALL be absent when the share is at or below the threshold.

#### Scenario: Most source is unsupported
- **WHEN** a repository's selected bytes are mostly an unsupported language
- **THEN** the verdict carries the qualifier sentence and the share fact while the tier is unchanged

#### Scenario: Unsupported source is marginal
- **WHEN** the unsupported byte share is at or below the threshold
- **THEN** no qualifier appears

#### Scenario: Two consumers print the qualifier
- **WHEN** the same qualified report is rendered for a human and serialized for a machine
- **THEN** both carry the analysis-owned qualifier bytes
