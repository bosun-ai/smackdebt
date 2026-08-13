## ADDED Requirements

### Requirement: README explains role classification and conflicts
The README SHALL name all six SourceRole values, which roles affect default
verdicts, precedence from configuration through fallback, same-level conflict
exit 2, language-generated markers, generic rules, and retained generated
directories.

#### Scenario: A user configures overlapping role rules
- **WHEN** they consult role documentation
- **THEN** it states the exact precedence and conflict outcome before they run analysis

### Requirement: README explains recovered advisory evidence
The README SHALL state that recovered Watch and High facts and dependency
context remain in JSON and `--all` but do not affect health, default output,
architecture verdicts, coupling, or diff verdicts.

#### Scenario: A user sees an advisory finding
- **WHEN** they compare default and detailed output
- **THEN** documentation explains why it appears only in detailed evidence

### Requirement: README explains static and history signal rules
The README SHALL distinguish `uses` from `module_ownership`, keep role and trust
separate, state that default architecture shows rated witnesses rather than
arbitrary edges, and document the three-shared-commit and 20% Jaccard coupling
threshold plus weak-observation visibility.

#### Scenario: Rust ownership no longer creates a cycle
- **WHEN** a user reads the architecture section
- **THEN** it explains why ownership is context rather than a dependency verdict edge

### Requirement: README documents exact rank and labels
The README SHALL list the rank sequence exactly as rating, signals at that
rating, total triggered signals, cognitive complexity, cyclomatic complexity,
logical lines, activity, path, and span. It SHALL state that unit kind and
non-primary role are shown and that terminal `repository root` maps to machine
path `.`.

#### Scenario: Two findings have the same rating
- **WHEN** a user wants to understand their order
- **THEN** documentation provides every comparison key in order

### Requirement: README presents JSON version 3 and exact examples
The README SHALL identify version 3 as the machine contract, link its checked
schema, and use executable examples that assert exact status, stderr, and all
declared stdout fragments in order.

#### Scenario: A documented example changes
- **WHEN** its result no longer matches the declared behavior
- **THEN** documentation acceptance fails with a reviewable difference
