## ADDED Requirements

### Requirement: History work has explicit resource limits

The system SHALL use one streamed Git history process per report, apply
backpressure while aggregating commits, and keep relationship storage
proportional to observed package pairs.

#### Scenario: A repository has many commits

- **WHEN** a complete correctness-checked codebase report reads its history
- **THEN** instrumentation records exactly one history process
- **AND** source analysis does not start an additional history process

#### Scenario: One commit changes many packages

- **WHEN** a generated commit touches many files and packages
- **THEN** each package is deduplicated before pair generation
- **AND** each unordered package pair is counted at most once for that commit

### Requirement: Evolutionary output is deterministic

The system SHALL order history summaries and coupling findings by stable report
identity after aggregation.

#### Scenario: Worker count changes

- **WHEN** the same generated history fixture is analyzed with one worker and
  automatic parallelism
- **THEN** terminal and JSON bytes are identical

