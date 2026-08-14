## MODIFIED Requirements

### Requirement: Architecture integrates with progressive reports
Report and JSON SHALL retain complete architecture findings, closed witnesses,
resolved relations, ownership, external references, unmatched references,
ambiguous references, role, trust, spans, counts, and indexes. Human repository,
package, directory, and file views SHALL select only linked architecture
findings and the grouped architecture resolution warning. Default SHALL show at
most three findings and `--all` SHALL show every linked finding. Each displayed
finding SHALL use a closed stored witness. Path selection SHALL NOT enable raw
incoming, outgoing, ownership, external, unmatched, ambiguous, advisory, or
other relation rows. Analysis/report aggregation SHALL keep architecture-finding
IDs unique within each scope owning list; index audits SHALL reject duplicates,
and terminal output SHALL visit each linked ID once without renderer de-duplication.

#### Scenario: Repository has many ordinary relations and no cycle finding
- **WHEN** default, `--all`, and path terminal views are rendered
- **THEN** `ARCHITECTURE` is absent apart from the grouped resolution warning when needed, while JSON retains complete relationship facts

#### Scenario: A user selects one package with several cycle findings
- **WHEN** default and `--all` terminal views read its existing architecture-finding links
- **THEN** default shows at most three findings and `--all` shows every linked finding, each once with a closed stored witness and no raw relation rows
