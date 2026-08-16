# verdict-policy Specification

## Purpose
TBD - created by archiving change add-verdict-policy. Update Purpose after archive.
## Requirements
### Requirement: Codebase verdicts use frozen tiers and sentences
Analysis SHALL own one codebase verdict tier per selected scope, using exactly
these frozen tier ids and sentences: `empty` with `Nothing was checked.`,
`clean` with `Clean. Ship it.`, `solid` with `Solid, with rough edges.`, `worn`
with `Worn in the usual places.`, `fights_back` with `This code fights back.`,
and `lost` with `The code is winning.` Tier ids SHALL be the stable contract for
machine consumers. Sentences SHALL be owned by analysis so every consumer prints
identical bytes, and no renderer SHALL compose its own sentence. The verdict
SHALL retain the counts it was computed from.

#### Scenario: A verdict is produced
- **WHEN** any codebase scope is analyzed
- **THEN** exactly one tier id and its exact sentence are available from analysis

#### Scenario: Two consumers print the verdict
- **WHEN** the same report is rendered for a human and serialized for a machine
- **THEN** both use the analysis-owned sentence bytes for the same tier id

### Requirement: Tier mapping uses integer permille arithmetic
Tier selection SHALL evaluate, in order: zero checked units selects `empty`;
zero High and zero Watch selects `clean`; zero High selects `solid`; a High
permille of at most 10 selects `worn`; a High permille of at most 50 selects
`fights_back`; any greater value selects `lost`. High permille SHALL be the
integer division of High count multiplied by 1000 by the checked unit count. No
floating-point value SHALL be produced anywhere in tier selection. A value
exactly at a boundary SHALL select the lower tier.

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

### Requirement: Architecture findings escalate the tier
At least one High architecture finding SHALL floor the codebase tier at `worn`,
and at least three SHALL floor it at `fights_back`. A floor SHALL only raise a
tier selected by permille mapping and SHALL never lower it. Escalation SHALL be
applied after mapping and SHALL be visible in the retained verdict facts.

#### Scenario: One package cycle exists in clean code
- **WHEN** every rated unit is healthy and one High architecture finding exists
- **THEN** the tier is `worn` rather than `clean`

#### Scenario: Three package cycles exist
- **WHEN** three High architecture findings exist and permille mapping selected `solid`
- **THEN** the tier is `fights_back`

#### Scenario: The mapped tier is already higher
- **WHEN** permille mapping selected `lost` and one High architecture finding exists
- **THEN** the tier remains `lost`

### Requirement: Diff verdicts reconcile all three comparison families
Analysis SHALL own one diff verdict tier per selected scope using exactly these
frozen tier ids and sentences: `no_debt_change` with `No debt changed.`, `better`
with `You made it better.`, `worse` with `You made it worse.`, and `mixed` with
`Better here, worse there.` The tier SHALL be decided from debt-diff membership
across source comparisons, architecture findings, and evolutionary findings
together. Any worse member SHALL make the tier worse, any better member SHALL
make it better, both SHALL make it `mixed`, and no member SHALL make it
`no_debt_change`. The verdict SHALL retain per-family counts so its facts can
name the family that moved. Every count SHALL be labeled with its word, and a
zero count SHALL be stated rather than omitted.

#### Scenario: Only a package cycle was introduced
- **WHEN** no source comparison moves debt and one package cycle is introduced
- **THEN** the tier is `worse` and its facts name the architecture family

#### Scenario: Debt moves in both directions
- **WHEN** one unit regressed and another improved
- **THEN** the tier is `mixed`

#### Scenario: Only healthy units were added
- **WHEN** a diff adds and removes healthy units and moves no rated debt
- **THEN** the tier is `no_debt_change`

#### Scenario: A count is zero
- **WHEN** the better count is zero and the worse count is one
- **THEN** both counts are retained with their words and neither is omitted

### Requirement: DebtDiffSelection defines human debt movement
Analysis SHALL own a per-scope selection of typed comparison and finding
identities that count as human debt movement. The selection SHALL contain
Regressed, Improved, Added at Watch or High, Removed at Watch or High, and
MetricChanged-while-rated source comparisons, plus architecture and evolutionary
findings that were introduced or removed. It SHALL NOT contain healthy added or
removed units, unchanged comparisons, ambiguous comparisons, or comparisons
whose file is fixture or generated source; those SHALL remain available in the
machine report. The selection SHALL store identities rather than copies, SHALL
feed the verdict and presentation, and SHALL be audited so a duplicate identity
within one scope's selection is rejected.

#### Scenario: A healthy unit is added
- **WHEN** a diff adds 204 healthy units
- **THEN** no selection member exists for them and they remain in the machine report

#### Scenario: A rated unit changes a measurement
- **WHEN** a unit rated Watch or High has a changed measurement
- **THEN** its comparison is a selection member

#### Scenario: A duplicate identity is produced
- **WHEN** one scope's selection would contain the same identity twice
- **THEN** the index-integrity audit fails

#### Scenario: Generated source changes
- **WHEN** a fixture or generated file regresses
- **THEN** it is absent from the selection and the verdict does not move

### Requirement: The worst offender is named with a resolved path
Analysis SHALL select the worst offender for a scope as the first entry of that
scope's finding rank, retaining its resolved repository-relative path string.
Its reason SHALL be `hot AND complex` when the finding belongs to a hotspot file
and `most complex` otherwise. When a scope has no ranked source finding but has
a package dependency cycle, the worst offender SHALL be that cycle's first
witness with reason `package dependency cycle`. When neither exists, the scope
SHALL have no worst offender.

Each reason SHALL carry a frozen machine identifier owned by analysis beside its
words, exactly: `hot_and_complex`, `most_complex`, and
`package_dependency_cycle`. These identifiers are the stable contract for
machine consumers the way tier ids are, and no renderer SHALL invent, rename, or
compose one.

#### Scenario: The top finding is in a hotspot file
- **WHEN** the first ranked finding's file is a hotspot
- **THEN** the worst offender carries that finding's resolved path and reason `hot AND complex`

#### Scenario: The top finding is cold
- **WHEN** the first ranked finding's file is not a hotspot
- **THEN** the reason is `most complex`

#### Scenario: Only a cycle exists
- **WHEN** a scope has no ranked source finding and one package cycle
- **THEN** the worst offender is the cycle's first witness with reason `package dependency cycle`

#### Scenario: Nothing is wrong
- **WHEN** a scope has no ranked finding and no cycle
- **THEN** no worst offender exists

#### Scenario: A machine consumer reads a reason
- **WHEN** a worst offender is serialized
- **THEN** its reason is one of the three frozen identifiers, taken from analysis rather than composed by the renderer

