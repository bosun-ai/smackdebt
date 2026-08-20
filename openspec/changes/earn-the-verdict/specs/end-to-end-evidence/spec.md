## MODIFIED Requirements

### Requirement: Acceptance tests invoke the real CLI process

The system SHALL test public behavior by starting the built command with an
explicit working directory, arguments, environment, terminal settings, and
worker policy and by asserting status, stdout, and stderr bytes.

The child environment SHALL be hermetic: acceptance invocations and fixture
git commands SHALL pin the home directory, the user configuration directory,
and the global and system git configuration to harness-owned locations
through one shared helper, so no machine-level ignore file or git setting can
change evidence bytes. Global-gitignore behavior SHALL have positive evidence:
a harness-written global ignore file provably excludes a fixture file.

#### Scenario: A successful codebase report runs

- **WHEN** the acceptance harness invokes the built command in a generated
  repository
- **THEN** the process exits with the documented status
- **AND** stdout exactly matches the expected report
- **AND** stderr is empty

#### Scenario: Invocation fails before a report exists

- **WHEN** the command receives an invalid ref or invalid argument
- **THEN** it exits with the documented nonzero status
- **AND** the product error appears on stderr
- **AND** stdout contains no partial report

#### Scenario: The developer machine has a global gitignore

- **WHEN** the same acceptance suite runs on machines with different user
  ignore files and git configuration
- **THEN** every asserted byte is identical because the child environment is
  pinned

#### Scenario: A pinned global ignore is honored

- **WHEN** the harness writes a global ignore pattern into the pinned
  configuration home
- **THEN** the matching fixture file is excluded from the report

## ADDED Requirements

### Requirement: The ratchet gate has exact acceptance evidence
Black-box evidence SHALL cover the gate: a clean gate exiting 0, a regressed
gate exiting 3 while naming the offending rows, a missing baseline exiting 2
with the exact documented stderr bytes, `--update` proven byte-stable and
idempotent, and gate `--json` validated against its checked schema.

#### Scenario: The gate regresses
- **WHEN** the acceptance fixture's debt exceeds its committed baseline
- **THEN** the real CLI exits 3 and stdout names each regressed row exactly

#### Scenario: The gate JSON is validated
- **WHEN** a gate result is emitted as JSON in acceptance
- **THEN** it validates against the gate schema before exact-byte approval
