## ADDED Requirements

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

## MODIFIED Requirements

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
