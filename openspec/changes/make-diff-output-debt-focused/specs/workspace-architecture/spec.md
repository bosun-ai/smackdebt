## MODIFIED Requirements

### Requirement: Report storage is flat and progressively addressable
The report SHALL store scopes, findings, diagnostics, and comparisons in flat
collections with stable typed indexes. Scope relationships SHALL support
repository, package, directory, and file traversal without recursive ownership.
Analysis/report aggregation SHALL retain the accepted unique typed IDs in each
owning scope link list and SHALL additionally own one private nonserialized
`DebtDiffSelection` per diff scope. It SHALL hold three lists with unique typed
IDs: trusted source debt comparisons, introduced/removed architecture-finding
comparisons, and introduced/removed evolution-finding comparisons. Recovered
comparisons SHALL remain outside all three lists in both human modes. Each
selection SHALL own one meaningful `DebtDiffCounts` value object derived only
from source IDs. Architecture and evolution IDs SHALL NOT contribute to it.
Analysis/report aggregation SHALL also aggregate each unique selected changed
file once for the human warning across read, parse, recovery, and ambiguous-unit
diagnostics. Index integrity SHALL reject a repeated ID within each typed list
or repeated changed-file warning link.

#### Scenario: Renderer shows a selected diff scope
- **WHEN** terminal receives a completed report and selected scope
- **THEN** it reads the three preselected typed lists and source-only `DebtDiffCounts` without copying the report, filtering trust, reclassifying, or scanning global comparison tables

#### Scenario: One typed selection list repeats an ID
- **WHEN** the same typed ID occurs twice in one source, architecture, or evolution list
- **THEN** report/index integrity fails before presentation

#### Scenario: One comparison contributes to ancestors
- **WHEN** a human debt comparison affects file, directory, package, and repository scopes
- **THEN** each separate owning scope list links its typed ID once

#### Scenario: One changed file has several incomplete-analysis reasons
- **WHEN** read, parse, recovery, or ambiguous-unit diagnostics refer to the same selected changed file
- **THEN** report aggregation links that file once for the scope warning while JSON retains every diagnostic

### Requirement: Report assembly belongs to analysis
Analysis SHALL expose one small report-construction interface that inserts
indexed facts, links retained facts, interns paths, aggregates a completed
report, constructs each private per-scope trusted `DebtDiffSelection` with three
typed lists, computes source-only `DebtDiffCounts`, and aggregates unique
selected changed-file warning links. Project
SHALL keep use-case-specific hierarchy and result assembly private. Terminal
SHALL consume that completed selection without trust policy,
reclassification, count changes, or project work.

#### Scenario: Project completes diff analysis work
- **WHEN** project orchestration has completed source, architecture, and evolution comparison facts
- **THEN** it passes them to report construction without building human presentation links itself or mutating a completed report
