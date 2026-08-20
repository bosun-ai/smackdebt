# source-discovery Specification

## Purpose
TBD - created by archiving change earn-the-verdict. Update Purpose after archive.
## Requirements
### Requirement: Discovery honors git ignore semantics
Discovery SHALL exclude source candidates using full git ignore semantics: the
analyzed root's `.gitignore`, nested `.gitignore` files at any depth,
`.git/info/exclude`, and the user's global gitignore. A pattern beginning with
`/` SHALL anchor at its ignore file's directory and SHALL NOT match at other
depths. A `!` negation SHALL re-include a previously excluded candidate. A
nested ignore file SHALL override an ancestor's decision for its own subtree.
`.gitignore` files SHALL be honored whether or not the analyzed root is inside
a git repository. When a subpath of a repository is analyzed, ignore files in
ancestor directories up to the repository root SHALL apply. Discovery SHALL NOT
read `.ignore` or `.rgignore` files and SHALL NOT skip a candidate solely for
being hidden.

#### Scenario: An anchored pattern is nested deeper
- **WHEN** the root `.gitignore` contains `/build.rs` and a file `sub/build.rs` exists
- **THEN** the root `build.rs` is excluded and `sub/build.rs` remains a source candidate

#### Scenario: A negation re-includes a file
- **WHEN** an ignore file excludes `*.rs` and a later line reads `!keep.rs`
- **THEN** `keep.rs` remains a source candidate while other `.rs` files are excluded

#### Scenario: A nested ignore file overrides its ancestor
- **WHEN** the root ignores a name and a nested `.gitignore` negates it for its subtree
- **THEN** the nested subtree keeps the file and the rest of the tree excludes it

#### Scenario: The directory is not a repository
- **WHEN** a directory without git metadata contains a `.gitignore`
- **THEN** its patterns still exclude matching candidates

#### Scenario: A repository subpath is analyzed
- **WHEN** the selected path is a subdirectory and an ancestor ignore file matches a candidate inside it
- **THEN** the candidate is excluded exactly as a whole-repository run would exclude it

### Requirement: Dependency directories are always excluded
Discovery SHALL always skip `.git`, `.hg`, `.svn`, `target`, `node_modules`,
and `vendor` directories. This exclusion SHALL apply regardless of ignore-file
content and SHALL win over `!` negations, because dependency directories are
promised excluded by default.

#### Scenario: A negation targets a dependency directory
- **WHEN** an ignore file contains `!node_modules/`
- **THEN** `node_modules` is still skipped and contributes no source candidate

### Requirement: Nested git checkouts are never analyzed
Discovery SHALL prune any directory below the analyzed root that contains a
`.git` entry, whether that entry is a directory or a file, so submodules,
embedded clones, and linked worktrees never contribute source candidates. The
analyzed root itself SHALL never be pruned. Each pruned checkout SHALL be
recorded as a diagnostic that names its repository-relative path, and the
count SHALL be visible in coverage.

#### Scenario: A linked worktree lives inside the repository
- **WHEN** a subdirectory contains a `.git` file pointing at another git directory
- **THEN** the subtree is pruned, contributes no findings, and one nested-repository diagnostic records its path

#### Scenario: An embedded clone lives inside the repository
- **WHEN** a subdirectory contains a `.git` directory
- **THEN** the subtree is pruned and recorded exactly as the worktree case

#### Scenario: The analyzed root is itself a repository
- **WHEN** the selected root contains its own `.git` entry
- **THEN** the root is walked normally and no nested-repository diagnostic is recorded for it

### Requirement: Configuration excludes use gitignore syntax
Patterns in the project configuration's exclude list SHALL be interpreted as
gitignore syntax anchored at the analyzed root: anchored patterns SHALL match
only at their stated depth and `!` negations SHALL re-include. Configuration
SHALL NOT execute commands or load code to decide exclusion. The always-skip
dependency-directory rule SHALL still win over configuration negations.

#### Scenario: A configuration pattern is anchored
- **WHEN** the exclude list contains `/generated`
- **THEN** the root-level `generated` directory is excluded and a nested `sub/generated` is not

#### Scenario: A configuration negation re-includes
- **WHEN** the exclude list contains `docs/**` followed by `!docs/spec.rs`
- **THEN** `docs/spec.rs` remains a source candidate

### Requirement: Discovery order is deterministic
Discovery SHALL walk the selected tree once, serially, visiting each
directory's entries in file-name order, and SHALL finish with one
repository-relative ordering of discovered files. The per-directory name order
SHALL govern which manifest is recorded first for a directory. Two walks of
the same tree SHALL yield identical order, and serial and parallel report runs
SHALL remain byte-identical because both index the same sorted inventory.
Symbolic links SHALL NOT be followed and SHALL remain recorded, and an
unreadable directory SHALL be recorded as a diagnostic rather than aborting
the walk.

#### Scenario: The same tree is walked twice
- **WHEN** discovery runs twice over an unchanged tree
- **THEN** both runs yield the same files in the same order

#### Scenario: A directory cannot be read
- **WHEN** one subdirectory denies permission
- **THEN** the walk completes, the directory is recorded as unreadable, and other candidates are unaffected

