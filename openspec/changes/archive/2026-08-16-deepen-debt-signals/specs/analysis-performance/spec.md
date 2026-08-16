## MODIFIED Requirements

### Requirement: Revised signal policy stays in the measured flow
Performance evidence SHALL measure role classification, recovery retention,
stable package construction, relation evidence, history filtering, window
filtering, hotspot derivation, stable-dependency evaluation, knowledge
concentration, size rating, orphan derivation, report construction, ranking,
de-duplication, and streaming output in the same correctness-checked invocation.
The new signals SHALL reuse existing passes and SHALL NOT add a file read, a
source traversal, or a Git process.

#### Scenario: A sample is timed
- **WHEN** public or private aggregate evidence is accepted
- **THEN** that invocation already passed semantics, schema, indexes, privacy, bytes, and exact work counts

#### Scenario: New signals are measured
- **WHEN** hotspots, stable-dependency findings, knowledge concentration, size findings, and orphan files are derived
- **THEN** exact work counts show the same inventory, read, parser, and Git process totals as before the signals existed

### Requirement: Revised tables retain hot-path limits
Package, advisory, relation, history, hotspot, size-finding, and orphan storage SHALL
reserve from known input counts. Rating, graph aggregation, coupling
threshold evaluation, hotspot derivation, stable-dependency evaluation, and
ranking hot loops SHALL not allocate after prepared storage is available. Every
new table SHALL be ordered by data-stable keys so serial and parallel runs
produce identical bytes.

#### Scenario: Generated and recovered context is retained
- **WHEN** it is excluded from verdicts but kept for detail
- **THEN** allocation and peak-memory evidence remain inside reviewed flow budgets

#### Scenario: New tables are ordered
- **WHEN** the same repository is analyzed with one worker and with automatic parallelism
- **THEN** hotspot, size-finding, and orphan rows appear in identical order and the public bytes match
