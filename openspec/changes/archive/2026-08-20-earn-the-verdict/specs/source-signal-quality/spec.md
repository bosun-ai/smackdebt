## MODIFIED Requirements

### Requirement: Generated directory names are not blanket ignores
Discovery SHALL NOT exclude a supported source candidate solely because a
directory name commonly indicates generated content. This rule SHALL hold
under the git-semantics ignore engine: only git ignore rules, explicit user
configuration excludes, the always-excluded dependency-directory list, and
the nested-checkout exclusion remove candidates; retained files receive a
role after inventory.

#### Scenario: Generated source is tracked in a common output directory
- **WHEN** the directory is not otherwise ignored
- **THEN** the file remains visible with its classified role

#### Scenario: A generated-sounding name is not ignored by git
- **WHEN** a tracked `generated/` directory matches no ignore rule, exclude, dependency-directory, or nested-checkout rule
- **THEN** its supported files are analyzed and classified by role rather than silently dropped
