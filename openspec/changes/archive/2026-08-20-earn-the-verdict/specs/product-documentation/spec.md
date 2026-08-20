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
eligible `uses` relation and no dependency path exists between the pair in
either direction, that a pair with no direct relation but a connecting
dependency path reads `no direct dependency` with `linked via <package>`
naming the first intermediate, that an indirect link never suppresses a
coupling finding, and that a coupling pair is reported once and never between
a scope and its own ancestor. The documented coupling example SHALL show one
genuinely unlinked pair and one indirect pair.

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

#### Scenario: A user sees an indirect pair
- **WHEN** the user reads the coupling documentation after seeing `linked via <package>`
- **THEN** the README explains the difference between a direct relation, an indirect path, and no path, and that findings are unaffected

### Requirement: README documents terminal presentation controls
Product documentation SHALL explain that severity, direction, and diagnostics are stated in
words, that glyphs and the tier bar are optional decoration resolved from
terminal detection, and that piped output is the same report in words with no
private-use codepoint. It SHALL explain `--color`, `NO_COLOR`, width behavior,
content-aware row stacking, `--top N` as a middle level of finding detail that
conflicts with `--json` and `--all`, `--all` as all useful debt, and JSON as
the complete machine-readable view of everything the terminal omits. It SHALL
state that there is no icon option, emoji mode, or theme setting.

The previously documented commitment that there is no ASCII fallback SHALL be
removed, because undecorated words-only output is now the defined behavior for
non-terminal and color-disabled output.

#### Scenario: User runs Smackdebt in automation
- **WHEN** the user reads output guidance
- **THEN** the README explains word-only piped output, how to force or disable decoration and ANSI styling, and that JSON remains unstyled and complete

#### Scenario: User looks for the fallback statement
- **WHEN** the user reads presentation documentation
- **THEN** no claim that ASCII or plain fallback is unavailable appears

#### Scenario: User wants more than three findings without everything
- **WHEN** the user reads the detail documentation
- **THEN** the README explains `--top N`, what it limits, and its conflicts

## ADDED Requirements

### Requirement: README documents the ratchet gate
The README SHALL document `smackdebt gate`: the committed baseline file, the
ratcheted time-invariant signals, the regression and improvement rules,
`--update`, `--baseline`, gate `--json` with its checked schema, and the
exit-code table gaining `3` for an exceeded gate baseline. The statement that
policy gates belong to a later release SHALL be removed. Gate examples SHALL
use the exact terminal vocabulary.

#### Scenario: A user sets up the gate
- **WHEN** the user reads the gate documentation
- **THEN** they can create a baseline, wire the gate into their checks, and interpret exit status 3

#### Scenario: A user looks for the deferred-gate statement
- **WHEN** the user reads the README after this change
- **THEN** no claim that policy gates belong to a later release appears

### Requirement: README states discovery rules and the license
The README SHALL state that discovery follows git ignore semantics — root and
nested `.gitignore`, `.git/info/exclude`, and the global gitignore, including
anchoring and negation — that dependency directories are always excluded and
win over negations, that nested git checkouts are never analyzed and are
disclosed, and that configuration excludes use gitignore syntax anchored at
the analyzed root. The repository SHALL carry a `LICENSE` file matching the
manifest's declared MIT license, and the README SHALL name it.

#### Scenario: A user asks why a file was excluded
- **WHEN** the user reads the discovery documentation
- **THEN** they can trace the exclusion to a gitignore rule, the dependency-directory list, a nested checkout, or a configuration pattern

#### Scenario: A user checks the license
- **WHEN** the user looks for licensing terms
- **THEN** the repository contains the MIT license text and the README names it
