## ADDED Requirements

### Requirement: Architecture documentation explains evolutionary ownership

`ARCHITECTURE.md` SHALL document that Git owns streamed history records,
analysis owns evolutionary algorithms and policy, project owns current-inventory
composition, and output only renders retained aggregate values.

#### Scenario: A maintainer reads the history design

- **WHEN** the maintainer opens `ARCHITECTURE.md`
- **THEN** the document identifies the owner of history I/O, rename identity,
  churn, coupling, concentration, comparison, and privacy enforcement
- **AND** it explains that contributor identities stop before the report seam

### Requirement: Architecture documentation states history limits

`ARCHITECTURE.md` SHALL explain current-file anchoring, current package
assignment, shallow or partial history, binary changes, and the absence of
historical package reconstruction.

#### Scenario: A maintainer evaluates a history claim

- **WHEN** the maintainer reads the evolution section
- **THEN** the document distinguishes exact retained facts from unavailable or
  excluded history
- **AND** it does not claim compiler, runtime, or historical build knowledge

