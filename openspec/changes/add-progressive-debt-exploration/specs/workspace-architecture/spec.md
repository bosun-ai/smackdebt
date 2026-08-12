## ADDED Requirements

### Requirement: Report paths have one owner
The completed report SHALL own each repository-relative path once in a flat path
table. Scopes, files, findings through files, comparisons through files, and
renderers SHALL use typed indexes to share those identities.

#### Scenario: One file contributes to several views
- **WHEN** a file contributes to file, directory, package, and repository output
- **THEN** the report stores its path once and those facts reach it through indexes

### Requirement: Scope selection is separate from report facts
`ProjectReport` SHALL identify an initial selected scope without storing mutable
navigation or terminal state in `Report`. Renderers SHALL accept an explicit
scope index.

#### Scenario: Another renderer is added
- **WHEN** a terminal interface or editor view consumes a completed root report
- **THEN** it can select retained scopes without adding filesystem, Git, parser, Rayon, or serialization concerns to analysis policy

### Requirement: Comparisons identify their file once
Every diff comparison SHALL refer to one file through a typed index. Scope
summaries SHALL refer to retained comparisons by index rather than copying
comparison values.

#### Scenario: One regression appears in ancestor summaries
- **WHEN** a comparison contributes to file, directory, package, and repository counts
- **THEN** the comparison is stored once and each applicable scope holds its index

### Requirement: Codebase and diff share hierarchy construction
Project orchestration SHALL use one internal hierarchy construction policy for
repository, package, directory, and file scopes in both report modes.

#### Scenario: A changed file belongs to a nested package
- **WHEN** codebase and diff reports cover the same repository-relative file
- **THEN** both assign it through the same package, directory, and file path structure
