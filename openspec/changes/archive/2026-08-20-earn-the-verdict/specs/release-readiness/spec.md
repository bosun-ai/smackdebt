## ADDED Requirements

### Requirement: Every commit runs the complete gate
The repository SHALL carry one continuous-integration workflow that runs the
complete check — formatting, lints, workspace tests, architecture checks,
performance tests, acceptance evidence, the ratchet gate once it exists,
strict OpenSpec validation, and the diff check — on every push and pull
request, on Linux and macOS, from a full-depth checkout so self-analysis and
the gate see complete history. Every tool the workflow installs SHALL be one
the complete check already needs, and tool versions SHALL match the
workspace's pinned versions.

#### Scenario: A commit is pushed
- **WHEN** any branch receives a push or a pull request is opened
- **THEN** the workflow runs the complete check on both platforms and the commit is red until every check passes

#### Scenario: The workflow drifts from the local gate
- **WHEN** the workflow installs a tool or version the complete check does not use
- **THEN** review rejects the drift so local and CI evidence stay the same gate
