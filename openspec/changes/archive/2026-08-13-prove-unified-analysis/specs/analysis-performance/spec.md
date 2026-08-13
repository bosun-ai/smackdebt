## ADDED Requirements

### Requirement: Complete-flow performance evidence preserves correctness

The system SHALL measure performance only for complete CLI flows whose terminal
or JSON result has already passed semantic, schema, and byte checks.

#### Scenario: A generated scaling workload is measured

- **WHEN** wall time, p95, peak memory, allocations, reads, and Git process
  counts are recorded
- **THEN** the same invocation first passes its exact correctness assertions
- **AND** the record includes workload identity, revision, dirty state,
  toolchain, host, source bytes, and supported-file count

### Requirement: Parallel evidence includes public byte equality

The system SHALL pair any parallel speed or memory evidence with one-worker and
automatic-parallel public-output comparison.

#### Scenario: Parallel execution is evaluated

- **WHEN** a complete analysis workload uses multiple workers
- **THEN** its terminal and JSON bytes match the one-worker result
- **AND** timing alone cannot make the performance check pass
