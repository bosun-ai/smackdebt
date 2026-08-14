## ADDED Requirements

### Requirement: Every selected file has one SourceRole
The system SHALL classify each selected file exactly once as `primary`, `test`,
`example`, `benchmark`, `fixture`, or `generated`. Primary SHALL be the fallback.

#### Scenario: A normal source file matches no rule
- **WHEN** no configured, language-generated, filename, or path rule matches
- **THEN** the file has the primary role

#### Scenario: Every role is present
- **WHEN** a fixture contains one file for each role
- **THEN** all six files remain separately addressable with their expected role

### Requirement: Role classification has fixed precedence and conflicts fail
Classification SHALL apply explicit declarative configuration before
language-owned generated markers, generic filename and path rules, and the
primary fallback. Several role matches at one precedence level SHALL stop the
CLI with exit 2, an exact stderr error, and empty stdout.

#### Scenario: Configuration overrides a generic rule
- **WHEN** explicit configuration assigns a path that a generic rule would classify differently
- **THEN** the configured role wins

#### Scenario: Same-level rules disagree
- **WHEN** two rules at the same precedence assign different roles
- **THEN** no report is produced and the CLI exits 2

### Requirement: Language-generated markers stay language-owned
Each language SHALL own recognition of generated source markers in its syntax
or metadata. Shared role policy SHALL own only generic filename and
repository-relative path rules and SHALL NOT inspect language grammar nodes.

#### Scenario: Generated markers differ by language
- **WHEN** two languages use different generated-source markers
- **THEN** each language implementation recognizes its marker without adding syntax knowledge to project composition

### Requirement: Generated directory names are not blanket ignores
Discovery SHALL NOT exclude a supported source candidate solely because a
directory name commonly indicates generated content. Git ignores, explicit user
ignores, and existing dependency-directory rules still apply; retained files
receive a role after inventory.

#### Scenario: Generated source is tracked in a common output directory
- **WHEN** the directory is not otherwise ignored
- **THEN** the file remains visible with its classified role

### Requirement: Roles have exact verdict participation
Primary, test, example, and benchmark source SHALL affect default code verdicts.
Fixture and generated source SHALL remain visible in coverage, JSON, path drill,
and `--all` but SHALL NOT affect default verdicts, architecture health, or
coupling findings.

#### Scenario: A fixture contains a High unit
- **WHEN** parsed fixture source exceeds a High threshold
- **THEN** its fact remains inspectable
- **AND** default health and findings do not change

#### Scenario: A benchmark contains a High unit
- **WHEN** parsed benchmark source exceeds a High threshold
- **THEN** it affects the default verdict and displays its non-primary role

### Requirement: Recovered facts are advisory
Recovered source SHALL retain measured units, Watch and High advisory facts,
dependency context, exact spans, role, and diagnostics in JSON and `--all`.
Recovered facts SHALL NOT contribute health, default findings, architecture
verdict edges, coupling findings, or diff verdicts and SHALL NOT count healthy.

#### Scenario: A recovered unit is High
- **WHEN** recovery still yields a High measurement
- **THEN** JSON and `--all` expose it as advisory
- **AND** default health and terminal findings exclude it

#### Scenario: A recovered dependency resolves
- **WHEN** recovered syntax identifies a repository target
- **THEN** the relation remains advisory context
- **AND** it never participates in a verdict graph

### Requirement: Failed source has no invented facts
Failed source SHALL retain role, coverage, and an exact diagnostic but SHALL NOT
emit invented units or dependency relations. Parsed, recovered, and failed
outcomes SHALL be shared across codebase, path, ref-diff, and worktree-diff
composition.

#### Scenario: Source analysis fails
- **WHEN** a supported file cannot produce an analysis
- **THEN** the report succeeds with failed coverage and no facts owned by that file
