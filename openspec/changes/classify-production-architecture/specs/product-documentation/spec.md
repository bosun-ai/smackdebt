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
resolved by manifest name and relations whose role is test, example, or
benchmark, and that a coupling pair is reported once and never between a scope
and its own ancestor.

The README SHALL further explain that architecture verdicts — package dependency
edges, package and file dependency cycles, instability, and stable-dependency
findings — are built from primary-role relations only, while test, example, and
benchmark relations stay complete in the machine report as context. It SHALL
state that a Rust reference declared under a `#[cfg(test)]` scope carries the
test role even when its file is production source, and that this is a syntactic
rule rather than an evaluation of configuration predicates. It SHALL state that a
`uses` relation between two files that already own each other through a Rust
module declaration is excluded from the file cycle graph, and that this exclusion
is limited to that pair so cycles between other files still report.

#### Scenario: Rust ownership no longer creates a cycle
- **WHEN** a user reads the architecture section
- **THEN** it explains why ownership is context rather than a dependency verdict edge, and why the imports between an owning pair are excluded from the file cycle graph

#### Scenario: A user asks why their test dependencies are absent from verdicts
- **WHEN** the user reads the architecture section after seeing test-role edges in JSON but no matching finding
- **THEN** the README explains that verdict graphs use primary-role relations only, that `#[cfg(test)]` scope assigns the test role inside production files, and that the excluded relations remain available as context and still explain change coupling

#### Scenario: A user analyzes a workspace
- **WHEN** the user reads how cross-package imports are resolved
- **THEN** the README explains declared-name matching, the unique-match rule, the retained diagnostic for duplicates, and what `no code dependency` claims
