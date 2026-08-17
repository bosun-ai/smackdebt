# source-signal-quality Specification

## Purpose
TBD - created by archiving change improve-analysis-signal-quality. Update Purpose after archive.
## Requirements
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
primary fallback. The primary fallback SHALL yield `test` for a file whose
module declarations are all test-scoped, as "A file declared only under a test
configuration is test source" states; no other precedence level SHALL be
affected by that rule. Several role matches at one precedence level SHALL stop
the CLI with exit 2, an exact stderr error, and empty stdout.

#### Scenario: Configuration overrides a generic rule
- **WHEN** explicit configuration assigns a path that a generic rule would classify differently
- **THEN** the configured role wins

#### Scenario: A declaration scope only refines the fallback
- **WHEN** a test-scoped declaration brings in a file that configuration, a generated marker, or a generic path rule already classifies
- **THEN** that classification wins and only a file that would fall back to primary becomes test

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

### Requirement: A file declared only under a test configuration is test source
Classification SHALL assign the role `test` to a file that another file declares
into the build, when the file has at least one such declaration, every one of
them is test-scoped, and its otherwise assigned role is the primary fallback.
A declaration SHALL be test-scoped when
the reference that declares it is test-scoped, or when the file that declares it
is itself test source by this rule, so the classification propagates through
chains of declarations. A file SHALL keep its role when any declaration of it is
not test-scoped, when it has no declaration at all, and whenever its role comes
from explicit configuration, a language-owned marker, or a generic filename or
path rule. The rule SHALL NOT create a relation, SHALL NOT change resolution or
trust, and SHALL reach the report before findings, ratings, coverage, and
history evidence read a role, so one file carries one role everywhere.

#### Scenario: A Rust module declared only under a test configuration
- **WHEN** a primary Rust file declares `#[cfg(test)] mod tests;` and the declared file exists beside it
- **THEN** the declared file is classified `test`, its outgoing relations carry the test role, and it does not enter an architecture verdict graph

#### Scenario: The same file is also declared outside a test configuration
- **WHEN** one declaration of a file is test-scoped and another declaration of the same file is not
- **THEN** the file keeps the primary role

#### Scenario: Declarations inside a test-declared file
- **WHEN** a test-declared file declares further modules without a configuration attribute
- **THEN** those declared files are test source as well

#### Scenario: Configuration keeps a declared file primary
- **WHEN** explicit configuration assigns a role to a file that a test-scoped declaration brings in
- **THEN** the configured role wins and the file is not reclassified
