## MODIFIED Requirements

### Requirement: Unified analysis evidence precedes publication
All three terminal change packages SHALL be allowed to exist before
implementation. `make-terminal-detail-relevant` SHALL be implemented only after
`make-terminal-verdict-clear` is reviewed and archived. It SHALL be reviewed and
archived before `make-diff-output-debt-focused` is implemented. The workspace
SHALL remain private and `prepare-first-release` SHALL remain blocked until all
three are archived with exact human-interface proof, unchanged complete JSON
and analysis behavior, public width and resource gates, installed behavior, and
three privacy-safe aggregate workload outcomes.

#### Scenario: Detail change is authored early
- **WHEN** this accepted change package exists before verdict implementation is archived
- **THEN** authoring is allowed but detail implementation does not begin

#### Scenario: Verdict change is archived
- **WHEN** `make-terminal-verdict-clear` has reviewed archived proof
- **THEN** this detail change can be implemented and reviewed

#### Scenario: Detail change is reviewed but not archived
- **WHEN** local detail behavior passes but this change remains active
- **THEN** diff-focused implementation and release preparation remain blocked

#### Scenario: All terminal changes are archived
- **WHEN** verdict, detail, and diff changes have reviewed archived proof in order
- **THEN** `prepare-first-release` is permitted to resume its own release evidence tasks
