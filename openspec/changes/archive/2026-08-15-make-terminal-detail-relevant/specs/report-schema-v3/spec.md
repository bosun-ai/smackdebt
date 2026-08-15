## MODIFIED Requirements

### Requirement: Version 3 has an executable schema and exact examples
The repository SHALL contain a checked JSON Schema for version 3. Codebase,
clean ref-diff, and mixed worktree-diff results SHALL validate schema, semantics,
indexes, privacy, and exact bytes from the same invocation. Executable index
validation SHALL reject a repeated typed ID within any one scope owning child,
source-finding, architecture-finding, evolution-finding, or diagnostic link
list. Accepted current fixtures SHALL already satisfy this rule and SHALL retain
their exact JSON bytes; validation SHALL NOT silently rewrite report data.

#### Scenario: An index points outside the package table
- **WHEN** acceptance validates the result
- **THEN** it fails before exact-byte approval

#### Scenario: An owning scope link list repeats an ID
- **WHEN** executable index validation reads the same typed ID twice in one owning list
- **THEN** it fails before exact-byte approval or terminal presentation

#### Scenario: Accepted current examples have unique owning links
- **WHEN** codebase and diff examples pass the stronger index validation
- **THEN** their approved JSON bytes remain unchanged
