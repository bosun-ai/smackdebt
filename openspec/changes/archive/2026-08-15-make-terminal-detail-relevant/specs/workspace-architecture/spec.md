## MODIFIED Requirements

### Requirement: Report storage is flat and progressively addressable
The report SHALL store scopes, findings, diagnostics, and comparisons in flat
collections with stable typed indexes. Scope relationships SHALL support
repository, package, directory, and file traversal without recursive ownership.
Analysis/report aggregation SHALL place each typed ID at most once within each
scope's owning child, source-finding, architecture-finding,
evolution-finding, and diagnostic link list. Report/index integrity audits
SHALL reject a repeated typed ID within one owning list before presentation or
serialization. A fact can remain linked once from each of several ancestor
scopes because those are separate owning lists.

#### Scenario: Renderer shows the next scope level
- **WHEN** a renderer receives a report and a selected scope
- **THEN** it can read that scope's direct children and retained findings without running analysis or copying the report

#### Scenario: One owning list repeats a typed ID
- **WHEN** report aggregation or a test fixture places the same typed ID twice in one scope owning link list
- **THEN** report/index integrity validation fails before any renderer can hide or rewrite the repeated link

#### Scenario: Ancestor scopes link the same fact
- **WHEN** one retained fact contributes once to more than one ancestor scope
- **THEN** each separate owning list can contain that typed ID once
