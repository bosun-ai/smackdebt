## MODIFIED Requirements

### Requirement: Unified analysis evidence precedes publication
Authoring all three terminal change packages before implementation begins SHALL
be allowed. Their implementation, review, and archive order SHALL be
`make-terminal-verdict-clear`, then `make-terminal-detail-relevant`, then
`make-diff-output-debt-focused`. The workspace SHALL remain private until
`make-terminal-verdict-clear` is
implemented, reviewed, and archived, followed in order by
`make-terminal-detail-relevant` and `make-diff-output-debt-focused`. The
`prepare-first-release` change SHALL remain blocked until all three terminal
changes are archived and their exact human interface, unchanged JSON and
analysis behavior, public width and resource gates, installed behavior, and
three privacy-safe workload outcomes pass from their reviewed implementation
revisions.

#### Scenario: Verdict work is reviewed but later terminal work remains
- **WHEN** `make-terminal-verdict-clear` passes but either following terminal change is not archived
- **THEN** `prepare-first-release` remains blocked and publication evidence is not finalized

#### Scenario: Later change packages are authored early
- **WHEN** detail and diff change documents exist before verdict implementation
- **THEN** authoring is accepted while their implementation still waits for the preceding change to be archived

#### Scenario: All terminal changes are archived
- **WHEN** the verdict, detail, and diff terminal changes are archived in order with reviewed proof
- **THEN** `prepare-first-release` is permitted to resume its own evidence and publication tasks

#### Scenario: Implementation is reviewed but not committed
- **WHEN** local terminal results look correct before the related implementation revision exists
- **THEN** clean release evidence is not recorded
