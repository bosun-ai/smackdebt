## MODIFIED Requirements

### Requirement: Problem rank is total and built once
Analysis SHALL order the problem table by rating descending, source priority,
claimed High count descending, hot before not hot, claimed finding count
descending, pattern in the frozen claiming order, the accepted finding rank of
the card's top claimed finding, anchor repository-relative path ascending, and
anchor start line ascending, in that order. Rating SHALL remain the strongest
priority.

Source priority SHALL put a card claiming any primary source finding first, a
card claiming no source finding second, and a card claiming source findings but
no primary source finding third. This keeps primary application debt ahead of
test, example, benchmark, fixture, and generated debt at the same rating while
giving architecture and history cards an explicit position. Source priority
SHALL NOT hide a card, change its visibility, rating, claims, counts, or verdict.
This specification SHALL NOT restate the keys of the finding rank, which
`hotspot-analysis` owns.

The order SHALL be total and data-stable, so serial and parallel runs produce
identical card order. The table SHALL be sorted once when the report is
finished, and no renderer SHALL sort, re-rank, or reorder it.

#### Scenario: Primary and benchmark cards share a rating
- **WHEN** a benchmark card claims more findings than a primary application card at the same rating
- **THEN** the primary application card appears first and the benchmark card remains visible lower in the ranked table

#### Scenario: Architecture has no source finding
- **WHEN** primary, architecture-only, and non-primary cards share a rating
- **THEN** they appear in that order before the remaining rank keys are considered

#### Scenario: Ratings differ
- **WHEN** a non-primary card is High and a primary application card is Watch
- **THEN** the High card appears first because rating remains the strongest priority

#### Scenario: Accepted terminal and machine views are rendered
- **WHEN** the public problem-pattern fixture is written in terminal and JSON form
- **THEN** both committed views carry the same primary, no-source, and non-primary card order

#### Scenario: Every rank key ties
- **WHEN** two cards tie on every key before the anchor
- **THEN** anchor path and anchor start line produce a stable order

#### Scenario: Serial and parallel runs cluster the same report
- **WHEN** the same selection is analyzed serially and in parallel
- **THEN** the card table is identical in content and order
