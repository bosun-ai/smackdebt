## MODIFIED Requirements

### Requirement: Unified analysis evidence precedes publication
The workspace SHALL remain private until all accepted analysis, terminal,
schema, evidence, performance, and documentation contracts pass. The terminal
changes SHALL be implemented, reviewed, and archived in order:
`make-terminal-verdict-clear`, `make-terminal-detail-relevant`, then
`make-diff-output-debt-focused`. `prepare-first-release` SHALL remain blocked
until all three are archived with reviewed evidence.

#### Scenario: Third terminal change is authored early
- **WHEN** its proposal, design, tasks, and deltas pass strict validation
- **THEN** implementation still waits for both earlier terminal changes to be reviewed and archived

#### Scenario: Release preparation is requested
- **WHEN** any of the three terminal changes is not archived with reviewed evidence
- **THEN** publication and release preparation remain blocked
