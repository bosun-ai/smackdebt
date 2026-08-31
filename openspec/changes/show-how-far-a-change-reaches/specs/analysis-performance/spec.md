## MODIFIED Requirements

### Requirement: Revised signal policy stays in the measured flow
Performance evidence SHALL measure role classification, recovery retention,
stable package construction, relation evidence, history filtering, window
filtering, hotspot derivation, stable-dependency evaluation, knowledge
concentration, size rating, orphan derivation, directory tree construction,
file pair accumulation, change amplification, propagation closures, path
probes, change-leakage joining, report construction, ranking,
de-duplication, and streaming output in the same correctness-checked invocation.
The new signals SHALL reuse existing passes and SHALL NOT add a file read, a
source traversal, or a Git process.

Every computation this change adds SHALL ride a pass that already runs — the
history stream callback, the architecture build, or the report finish — and
SHALL NOT record an algorithm pass of its own. Exact `algorithm_passes` totals
are asserted per flow in acceptance and in the performance report check, so a
new pass would be a behavior change disguised as a measurement.

#### Scenario: A sample is timed
- **WHEN** public or private aggregate evidence is accepted
- **THEN** that invocation already passed semantics, schema, indexes, privacy, bytes, and exact work counts

#### Scenario: New signals are measured
- **WHEN** hotspots, stable-dependency findings, knowledge concentration, size findings, and orphan files are derived
- **THEN** exact work counts show the same inventory, read, parser, and Git process totals as before the signals existed

#### Scenario: The change-reach signals are measured
- **WHEN** file pairs, amplification, closures, reach, core size, and leakage findings are derived
- **THEN** the inventory, read, parser, Git process, and algorithm pass totals equal the values asserted before this change

## ADDED Requirements

### Requirement: Change graph and reachability work has fixed bounds
Every computation this change adds SHALL have a stated bound that holds
independently of repository shape, and each bound SHALL be a named integer
constant so review can move it in one place:

- Pair accumulation SHALL be skipped for a commit touching more than the
  bulk-commit file count, so no single commit can contribute a quadratic burst
  of pairs, and SHALL create no new pair key beyond the pair storage limit.
  Both events SHALL be counted and disclosed rather than silently dropped.
- A per-package file closure SHALL be skipped above the closure node limit, so
  the transient bit set stays within a few megabytes.
- Exact per-file reach SHALL be computed for at most the candidate limit of
  files, one reverse breadth-first search each.
- Each path probe SHALL visit at most the probe node budget and SHALL answer
  undecided when it exhausts it.

Graph traversals added by this change SHALL be stack-safe on adversarial shapes,
proven the way the accepted strongly-connected-component implementation is
proven, so a two-hundred-thousand node chain completes rather than overflowing.

A public workload profile `evolution-wide` SHALL exist beside the existing
profiles, covering roughly two thousand files across fifty packages with roughly
forty commits that produce real cross-directory pairs, several provably
unlinked pairs, and one bulk commit, so every new bound is exercised by a
measured workload rather than only by unit tests. The existing dense-evolution
profile SHALL serve as the bulk-guard proof, where one commit touching a hundred
files produces one bulk commit and no pair. The workload identity, its recorded
counts, and its budgets SHALL follow the accepted workload evidence rules.

The machine-report performance check SHALL validate the file pair table the way
it validates the existing tables: bounds on counts, the lower file index first,
shared commits never exceeding union commits, a directory distance of at least
one, and no finding below the detector floors.

#### Scenario: One commit touches a hundred files
- **WHEN** the dense-evolution workload is measured
- **THEN** the bulk commit count is one, no pair is accumulated from that commit, and every other history value is unchanged

#### Scenario: A wide repository is measured
- **WHEN** the `evolution-wide` workload runs through the complete correctness-checked flow
- **THEN** its wall time, peak memory, allocations, reads, and Git process counts are recorded against its own workload identity

#### Scenario: A long dependency chain is closed over
- **WHEN** a two-hundred-thousand node chain is reduced and closed
- **THEN** the traversal completes without exhausting the stack

#### Scenario: The pair table is validated
- **WHEN** the performance report check reads a result carrying file pairs
- **THEN** every row has its lower file index first, shared commits at most union commits, a distance of at least one, and every leakage finding is above its floors
