## MODIFIED Requirements

### Requirement: Architecture documents the terminal presentation boundary
`ARCHITECTURE.md` SHALL describe CLI-owned output choices, borrowed scope
selection through existing child/finding/diagnostic links, one-pass relevance
filtering, content-aware writing, semantic styling, and unchanged complete JSON
serialization. It SHALL define default limits, useful-debt `--all`, path as
scope-only selection, unique owning sections, and JSON-only raw/context families.

#### Scenario: Engineer changes terminal detail
- **WHEN** selection, section presence, or `--all` behavior changes
- **THEN** the architecture guide directs fact creation and links to analysis, relevance selection to output policy, width/color to CLI and writer seams, and complete serialization to JSON

### Requirement: Architecture documentation separates analysis and presentation
Architecture documentation SHALL place rank keys, verdict inclusion, advisory
state, stable order, and unique typed IDs in each scope owning link list before
rendering. It SHALL assign duplicate-ID rejection to report/index integrity
audits. It SHALL describe terminal filtering as one borrowed pass over the
selected scope's completed unique links without renderer de-duplication,
identity mapping, a second selected-ID collection, role/trust arithmetic, large
clones, repeated global-table scans, filesystem, Git, parser, analysis, or
worker work.

#### Scenario: Human output omits retained context
- **WHEN** raw relationships, weak history, activity, or concentration stay out of terminal default, `--all`, and path views
- **THEN** the guide explains that report and JSON facts remain complete and only terminal presentation selection changed

#### Scenario: One typed ID appears twice in an owning list
- **WHEN** a maintainer follows the documented integrity seam
- **THEN** analysis/report index validation rejects it before rendering rather than silently de-duplicating it in output
