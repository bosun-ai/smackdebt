## MODIFIED Requirements

### Requirement: README explains static and history signal rules
The README SHALL distinguish `uses` from `module_ownership`, keep role and trust
separate, state that default architecture shows rated witnesses rather than
arbitrary edges, and document the three-shared-commit and 20% Jaccard coupling
threshold plus weak-observation visibility. It SHALL explain that a
cross-package reference written against a package's declared manifest name
resolves internally when exactly one package in the repository declares that
name, that a duplicate declared name stays unresolved with a diagnostic, and
that Smackdebt reads declared names only and never executes build
configuration. It SHALL state that `no code dependency` means no trusted
eligible `uses` relation exists in either direction, including relations
resolved by manifest name, and that a coupling pair is reported once and never
between a scope and its own ancestor.

#### Scenario: Rust ownership no longer creates a cycle
- **WHEN** a user reads the architecture section
- **THEN** it explains why ownership is context rather than a dependency verdict edge

#### Scenario: A user analyzes a workspace
- **WHEN** the user reads how cross-package imports are resolved
- **THEN** the README explains declared-name matching, the unique-match rule, the retained diagnostic for duplicates, and what `no code dependency` claims
