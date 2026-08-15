## MODIFIED Requirements

### Requirement: Default sections show only relevant decisions
Default repository, package, directory, and file terminal views SHALL apply the
same relevance policy. `QUALITY` SHALL always use the accepted checked-source
verdict: the sole `Nothing was checked.` line for zero checked units, otherwise
the grouped checked count and attention percentage followed by exact High/Watch
counts or `No findings.`. `AREAS` SHALL appear only for several debt-bearing
child areas and SHALL show at most five in existing stable order. `FINDINGS`
SHALL show at most three existing ranked source findings linked to the selected
scope. `ARCHITECTURE` SHALL show at most three linked architecture findings,
each with a closed stored witness. `HISTORY` SHALL show at most three linked
actionable evolution findings. Real source gaps and architecture resolution
SHALL retain their grouped warnings. At most one discover line SHALL contain
only the Discover glyph and the first displayed debt-bearing child's command.
Every optional section with no selected row SHALL be absent.

Each human history row SHALL use `<left> ↔ <right> changed together in <shared>
of <union> commits · <similarity>% · not linked in code` with the Watch glyph
and no repeated status word. The old final dependency phrase SHALL be absent.

#### Scenario: Repository has every useful finding family
- **WHEN** default repository output contains many linked facts
- **THEN** it shows at most five debt-bearing areas, three ranked source findings, three architecture findings with closed witnesses, three actionable history findings, grouped warnings, and one discover command

#### Scenario: A package directory or file is selected
- **WHEN** default output selects that scope
- **THEN** the same relevance limits and section-presence rules apply without enabling another fact family

#### Scenario: An optional section selects no rows
- **WHEN** architecture, history, areas, findings, warnings, or discover guidance has no relevant selected fact
- **THEN** its heading, table, and placeholder are absent

### Requirement: Human output removes repeated and internal facts
Every human terminal view SHALL omit healthy rows, raw resolved edges,
ownership rows, external references, unmatched rows, ambiguous rows, weak or
explained coupling observations, package/file activity rows, contributor
concentration, history coverage fields, history processing facts, and repeated
facts. Source finding commit count, closed witness evidence, and actionable
coupling shared commits, union commits, and similarity SHALL remain. Human
history SHALL end actionable coupling evidence with `not linked in code` and
SHALL NOT use `no code dependency`. Cognitive, cyclomatic, and
statement values SHALL remain on applicable source findings.

#### Scenario: A report retains every JSON fact family
- **WHEN** default, `--all`, and path terminal views are rendered
- **THEN** each shows only useful debt evidence once while excluded raw and contextual facts remain absent

### Requirement: Detailed and path views remain useful
`--all` SHALL mean all useful debt. It SHALL remove default count limits only
for debt-bearing child areas; existing scope-linked retained Watch and High
source findings, including non-primary role and advisory qualifiers already
attached to them; linked architecture findings with closed stored witnesses;
linked actionable evolution findings; and real failed/recovered diagnostics.
It SHALL NOT show healthy rows or any raw relationship, weak/explained history,
activity, concentration, history-coverage, or processing row.

A selected path SHALL change selected scope only and SHALL apply the same
default relevance policy unless `--all` is also supplied. Path selection SHALL
NOT enable raw incoming/outgoing uses, ownership, external, unmatched,
ambiguous, weak history, explained history, activity, concentration, or healthy
detail. The grouped architecture resolution warning SHALL be the only human
representation of unmatched and ambiguous imports.

Role and trust SHALL qualify an already selected source finding but SHALL NOT
alter verdict arithmetic, reclassify the finding, or create another terminal
inclusion rule. Analysis/report aggregation SHALL provide unique typed IDs in
each selected scope's owning child, source-finding, architecture-finding,
evolution-finding, and diagnostic link lists. Terminal output SHALL trust those
completed lists and visit each linked ID once without a de-dup set, second
selected-ID collection, identity mapping, or silent data rewrite. A fact can
appear once in each separately rendered ancestor scope; within one rendered
scope and owning section, each linked ID SHALL appear once.

#### Scenario: User requests all useful debt
- **WHEN** `--all` is supplied at repository, package, directory, or file scope
- **THEN** limits are removed for every linked useful-debt family and excluded raw, contextual, processing, and healthy families remain absent

#### Scenario: User selects a path without all detail
- **WHEN** a package, directory, or file path is supplied without `--all`
- **THEN** selection changes scope but keeps the same default limits and never enables raw relationship, weak history, or activity rows

#### Scenario: Qualified source finding is selected
- **WHEN** an existing linked Watch or High finding has a non-primary role or advisory trust
- **THEN** `--all` shows the finding once with its existing qualifiers without changing health or verdict arithmetic

#### Scenario: One fact is linked by separate ancestor scopes
- **WHEN** those scopes are rendered separately
- **THEN** each scope can show its one linked ID without renderer de-duplication across reports

### Requirement: Machine and analysis interfaces do not change
The system SHALL preserve JSON version 3 bytes and complete retained tables,
report facts and links, finding rank, analysis and role/trust policy, CLI flags,
exit behavior, serial and automatic behavior, live work counts, allocations,
and measured resource behavior unchanged through terminal detail reduction.
Index audits SHALL reject a duplicate typed ID within one scope owning list.
Accepted current fixtures SHALL pass that audit without rewriting data and
SHALL retain exact JSON bytes.

#### Scenario: Human selection becomes narrower
- **WHEN** the same dense fixture is rendered as default terminal, terminal `--all`, a path view, and JSON
- **THEN** only intended human terminal bytes differ while JSON completeness, analysis, work, and resource checks still pass

#### Scenario: One owning list contains a duplicate ID
- **WHEN** report/index integrity is checked
- **THEN** the audit fails before terminal or JSON rendering can hide or rewrite the duplicate
