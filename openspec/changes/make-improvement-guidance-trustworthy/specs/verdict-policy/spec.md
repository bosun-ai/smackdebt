## MODIFIED Requirements

### Requirement: Diff verdicts reconcile all three comparison families
Analysis SHALL own one diff verdict tier per selected scope using exactly these
frozen tier ids and sentences: `no_debt_change` with `No debt changed.`,
`better` with `Debt decreased.`, `worse` with `Debt increased.`, and `mixed`
with `Debt increased in some places and decreased in others.` The tier SHALL be
decided from debt-diff membership across source comparisons, architecture
findings, and evolutionary findings together. Any worse member SHALL make the
tier worse, any better member SHALL make it better, both SHALL make it `mixed`,
and no member SHALL make it `no_debt_change`. The verdict SHALL retain
per-family counts so its facts can name the family that moved. Every count SHALL
be labeled with its word, and a zero count SHALL be stated rather than omitted.

The tier identifiers, debt membership, direction rules, and per-family counts
SHALL remain unchanged when the sentences change. The sentences SHALL be owned
by analysis so terminal and JSON output use the same bytes.

#### Scenario: Only a package cycle was introduced
- **WHEN** no source comparison moves debt and one package cycle is introduced
- **THEN** the tier is `worse`, its sentence is `Debt increased.`, and its facts name the architecture family

#### Scenario: Debt moves in both directions
- **WHEN** one unit regressed and another improved
- **THEN** the tier is `mixed` and its sentence is `Debt increased in some places and decreased in others.`

#### Scenario: Only healthy units were added
- **WHEN** a diff adds and removes healthy units and moves no rated debt
- **THEN** the tier is `no_debt_change` and its sentence is `No debt changed.`

#### Scenario: Debt only decreases
- **WHEN** one debt-bearing unit is removed and no debt increases
- **THEN** the tier is `better` and its sentence is `Debt decreased.`

#### Scenario: A count is zero
- **WHEN** the better count is zero and the worse count is one
- **THEN** both counts are retained with their words and neither is omitted

### Requirement: The verdict qualifies unsupported coverage
Analysis SHALL compare the selected scope's analyzed file count with its
selected source file count. Whenever analyzed files are fewer than selected
source files, the verdict SHALL carry the frozen qualifier sentence
`Not all source was checked.`, the frozen detail sentence
`<analyzed> of <selected> source files were analyzed.`, both integer file
counts, the unsupported byte share in integer permille, and the largest
unsupported language by bytes when one exists. There SHALL be no file-count,
byte-share, or percentage threshold for creating the qualifier.

Analyzed files SHALL retain the accepted coverage meaning: clean, recovered,
and context files count as analyzed; unsupported and failed files do not. The
qualifier SHALL never change the selected tier, its counts, its worst offender,
or graph evidence. Both sentences SHALL be owned by analysis so every consumer
prints identical bytes. A fully analyzed selection SHALL carry no qualifier.

#### Scenario: Most source is unsupported
- **WHEN** a repository's selected bytes are mostly an unsupported language
- **THEN** the verdict carries both qualifier sentences, both file counts, the byte share, and the largest unsupported language while the tier is unchanged

#### Scenario: Unsupported source is marginal
- **WHEN** one of one thousand selected source files is unsupported
- **THEN** the qualifier exists and its detail states `999 of 1000 source files were analyzed.`

#### Scenario: Analysis fails for one file
- **WHEN** one selected supported file fails analysis and no language is unsupported
- **THEN** the qualifier carries the exact file counts and unsupported share, and carries no invented largest unsupported language

#### Scenario: Selection is complete
- **WHEN** analyzed files equal selected source files
- **THEN** no qualifier appears

#### Scenario: Two consumers print the qualifier
- **WHEN** the same incomplete report is rendered for a human and serialized for a machine
- **THEN** both carry the two analysis-owned sentences and exact selected and analyzed counts
