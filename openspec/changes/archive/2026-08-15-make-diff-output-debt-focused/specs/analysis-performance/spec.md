## MODIFIED Requirements

### Requirement: Output streams from one report
Terminal and JSON output SHALL write directly to `io::Write` from one borrowed
completed report without cloning source, finding, relationship, history, or
comparison collections. Human diff terminal output SHALL read the private
per-scope three typed `DebtDiffSelection` lists and source-only `DebtDiffCounts`
directly. Analysis SHALL exclude fixture, generated, and recovered comparisons
before completing those lists while preserving complete JSON facts. Output
SHALL NOT apply role/trust
policy, allocate an identity
map, de-dup set, second selected-ID collection, or second report; reclassify
comparisons; or scan global comparison tables. JSON SHALL stream all complete
facts and SHALL ignore private nonserialized selection objects.

#### Scenario: One completed diff report renders in both formats
- **WHEN** terminal and JSON output are requested in separate equivalent runs
- **THEN** terminal visits each selected ID once and JSON streams complete role/coverage/measurement/comparison/diagnostic facts without role/trust filtering, report clones, or presentation scans

### Requirement: Rating and aggregation avoid new heap allocation
Rating one unit and aggregating one retained item SHALL perform no heap
allocation when storage is already reserved. All three typed selection lists
and selected changed-file warning links SHALL be reserved before aggregation.
Linking one trusted typed ID into its applicable reserved list, updating
`DebtDiffCounts` only for source IDs, and counting each changed file once across
diagnostic kinds SHALL perform no new heap allocation.

#### Scenario: Allocation-instrumented selection aggregation runs
- **WHEN** one debt comparison is linked into reserved applicable scopes
- **THEN** the instrumented aggregation region records zero heap allocations, only a source ID updates the scope count, and repeated reasons do not repeat a changed file

### Requirement: Scope rendering performs no project work
Selecting or rendering any codebase or diff scope SHALL consume only completed
report links and SHALL perform no discovery, filesystem read, Git work, parsing,
analysis, worker scheduling, or per-row global-table scan. Diff rendering SHALL
read private unique typed lists and source-only `DebtDiffCounts` without
role/trust filtering, reclassification, count changes, repair, or a second collection.

#### Scenario: Dense diff scope renders default and all modes
- **WHEN** work instrumentation surrounds terminal selection and writing
- **THEN** each applicable selected ID is visited once and trust-filter, project, global-scan, and second-collection counters remain zero

### Requirement: Progressive links preserve measured report limits
Finding links and complete/private typed comparison lists SHALL use reserved storage;
path indexes, complete diff counts, and `DebtDiffCounts` SHALL do the same
and index existing facts without cloning path or finding payloads. Generated
scaling evidence SHALL measure allocation and peak memory and SHALL reject N+1
ancestry work or repeated global scans.

#### Scenario: Large diff hierarchy is aggregated
- **WHEN** many relevant and contextual comparisons contribute to nested scopes
- **THEN** complete facts remain once, trusted IDs link once per applicable scope list, only source IDs update `DebtDiffCounts`, and measured allocation and memory stay within reviewed limits
