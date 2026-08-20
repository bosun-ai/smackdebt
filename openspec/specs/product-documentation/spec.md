# product-documentation Specification

## Purpose
TBD - created by archiving change add-product-foundation. Update Purpose after archive.
## Requirements
### Requirement: README explains the product through its two user questions
The project SHALL provide a root `README.md` that explains codebase health and ref comparison in simple language.

#### Scenario: New user reads the introduction
- **WHEN** a user opens the repository README
- **THEN** the introduction states the two questions Smackdebt answers and explains progressive path discovery

### Requirement: README provides complete command examples
The README SHALL document installation, codebase analysis, path drill-down, ref
comparison, history configuration, and JSON output with realistic commands and
sample reports that use the exact short terminal sections and glyphs.

#### Scenario: User follows the codebase example
- **WHEN** a user reads the codebase analysis section
- **THEN** the README shows a no-argument command, relevant output, and one glyph-plus-command line for deeper inspection

#### Scenario: User follows the diff example
- **WHEN** a user reads the ref comparison section
- **THEN** the README shows default-ref discovery, relevant Worse, Better, and Changed findings, and path drill-down without internal report terms

### Requirement: README explains ratings without false precision
The README SHALL publish each default health threshold, explain how Git activity affects hotspot order, and state that Smackdebt does not calculate one repository score.

#### Scenario: User evaluates a finding
- **WHEN** a user reads a high or watch finding
- **THEN** the README provides enough information to trace the rating to cognitive complexity, cyclomatic complexity, or function size

### Requirement: README states support and limits
The README SHALL list supported languages, package discovery inputs, failure exit codes, optional configuration, privacy behavior, upstream attribution, and links to architecture and OpenSpec artifacts.

#### Scenario: User checks whether a repository is supported
- **WHEN** a user reads the support sections
- **THEN** the README identifies the initial languages and explains how unsupported or failed files affect coverage

### Requirement: README reflects private development installation
Before registry publication is authorized, the README SHALL NOT claim that
`cargo install smackdebt` works from the registry. It SHALL document local-path
installation or clearly label registry installation as planned behavior.

#### Scenario: Developer follows current installation instructions
- **WHEN** a developer follows the README before publication
- **THEN** the documented command installs from the checked-out CLI crate without requiring published workspace libraries

### Requirement: README lists only verified language support
The README SHALL list C/C++, Java, JavaScript/JSX, Python, Rust, TypeScript/TSX,
Ruby, and Vue as initial supported languages after their requirements are
implemented. It SHALL distinguish owned Ruby and Vue analysis from the temporary
upstream-backed set and SHALL NOT list Kotlin as supported.

#### Scenario: User checks a mixed Ruby and Vue repository
- **WHEN** the user reads the language section
- **THEN** the README explains that Ruby methods and Vue script and template regions receive source measurements

#### Scenario: User checks Kotlin support
- **WHEN** the user reads the initial language list
- **THEN** Kotlin is absent and unsupported files are described as visible coverage gaps

### Requirement: README states JSON detail retention
The README SHALL state that JSON retains every Watch and High finding and all scope
summaries, while healthy units are represented through aggregate counts. It
SHALL also state that JSON is the complete view of every fact the terminal
omits, including raw dependency edges, external references, churn detail, weak
coupling, hotspots, size findings, and orphan files.

#### Scenario: Integration author chooses JSON output
- **WHEN** an integration needs all debt findings
- **THEN** the README makes clear that JSON is complete for Watch and High findings but not a full healthy-unit index

#### Scenario: A user misses a row the terminal removed
- **WHEN** they look for a row that human output no longer prints
- **THEN** the README directs them to the version-4 JSON table that retains it

### Requirement: README explains package-root grouping
The README SHALL state that co-located manifests form one report package and
that each source file belongs to its nearest package-root directory once.

#### Scenario: Repository mixes ecosystems in one directory
- **WHEN** a user reads discovery behavior for a directory with several manifests
- **THEN** the README explains why the report shows one package rather than duplicate ecosystem packages

### Requirement: README demonstrates debt distribution
The README SHALL show the responsive codebase dashboard with exact child High,
Watch, debt-share, and local-rate values, rate bars, leading findings, and the
next drill command.

#### Scenario: New user follows progressive exploration
- **WHEN** the user reads the codebase example
- **THEN** the example explains the quality summary and moves from repository to child area to file using exact, internally consistent counts

### Requirement: README demonstrates progressive diff exploration
The README SHALL show a responsive diff dashboard with child Worse, Better,
Changed, and share values, share bars, and meaningful unit changes.

#### Scenario: User locates a worktree regression
- **WHEN** the user reads the diff example
- **THEN** the example shows how to identify the affected area and drill to its concise detailed comparison

### Requirement: README explains share and completeness
The README SHALL define codebase and diff share denominators, integer rounding,
path-limited scope, default terminal limits, `--all`, and complete JSON behavior.

#### Scenario: User interprets a percentage
- **WHEN** a displayed child share is rounded
- **THEN** the README directs the user to exact counts and explains why displayed shares may not sum to 100 percent

#### Scenario: User needs every retained result
- **WHEN** the default terminal view omits rows or details
- **THEN** the README explains terminal `--all` and that JSON is always complete

### Requirement: Product examples explain share and rate
Product documentation SHALL show progressive area output and distinguish an
area's share of selected debt from its local attention rate.

#### Scenario: User reads the root example
- **WHEN** the README introduces the codebase report
- **THEN** it explains how contribution and concentration support the next drill decision

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

### Requirement: Product documentation explains evolutionary signals

The README SHALL explain churn, package change coupling, contributor count,
contributor concentration, and unexplained-coupling Watch findings in plain
product language.

#### Scenario: A user interprets evolution output

- **WHEN** the user reads the README and an example report
- **THEN** the user can distinguish present code health, static architecture,
  and change-history evidence
- **AND** the user is not told that any descriptive value is automatically bad

### Requirement: Product documentation states privacy and coverage behavior

The README SHALL state that analysis stays local, contributor identities are
not reported, and unavailable or incomplete history is shown explicitly.

#### Scenario: A user analyzes a shallow repository

- **WHEN** the user consults history documentation
- **THEN** the documented output matches the incomplete-history diagnostic
- **AND** it explains which source and architecture results still remain usable

### Requirement: Documented command examples are checked
The README SHALL mark runnable console examples and associate each with a named
public generated fixture, expected status, and exact relevant output fragments.

#### Scenario: Documentation tests run
- **WHEN** documentation validation reads runnable README examples
- **THEN** it executes them through the built CLI and matches status, sections, glyphs, commands, stdout, and stderr in order

### Requirement: Documentation covers the unified result
The README SHALL show how one command answers code and architecture questions, opening
with the verdict block — scope, tier sentence, word-labeled counts, and worst
offender — followed by `AREAS`, `FINDINGS`, `ARCHITECTURE`, `HISTORY`, and
`WARNINGS` sections, with empty optional sections omitted. It SHALL list the
frozen codebase and diff tier ids with their sentences, state that a clean diff
prints the verdict line only, and state that every count is labeled with its
word including zero counts.

#### Scenario: A user reads the main examples
- **WHEN** the user follows documented default, path, and diff examples
- **THEN** the examples lead with the verdict, label every count, omit empty sections, and do not present a combined score

#### Scenario: A machine consumer reads the documentation
- **WHEN** an integration author decides what to key on
- **THEN** the README names the tier ids as the stable vocabulary and points to JSON for complete data

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

### Requirement: README documents exact rank and labels
The README SHALL list the rank sequence exactly as rating, role class, hot
state, signals at that rating, total triggered signals, cognitive complexity,
cyclomatic complexity, logical lines, activity, path, and span. It SHALL name
that measurement `logical lines`, matching the architecture documentation and
the machine contract, and SHALL NOT call it `statements` in the rank sequence.
It SHALL state that primary
source precedes non-primary source at equal rating and that hot state decides
next, that hot means a rated file whose windowed touch count reaches the minimum
touch count, that non-primary source remains visible below primary source and is
still named as the worst offender when nothing else is rated, that unit kind and
non-primary role are shown, and that terminal `repository root` maps to machine
path `.`.

#### Scenario: Two findings have the same rating
- **WHEN** a user wants to understand their order
- **THEN** documentation provides every comparison key in order, beginning with role class and hot state before either signal count

#### Scenario: Test debt appears below production debt
- **WHEN** a user compares a primary and a test finding with the same rating
- **THEN** documentation explains the role class key and states that test debt stays visible

#### Scenario: A user compares the documented rank with the machine contract
- **WHEN** the user reads the README rank sequence beside the serialized measurement names
- **THEN** both call the measurement logical lines and the README rank sequence does not say `statements`

### Requirement: README presents JSON version 4 and exact examples
The README SHALL identify version 4 as the machine contract, link its checked schema, and
describe the denormalized head — `verdict` with tier, sentence, and mode, and
`summary` with counts and up to three fully resolved worst entries — as the way
to answer the common question without joining tables. It SHALL state that every
serialized value is an integer or a string, that similarity and concentration
are published as their integer operands rather than as ratios, and that no
documented text promises a serialized similarity or ratio value. Its executable
examples SHALL assert exact status, stderr, and all declared stdout fragments in
order.

#### Scenario: A documented example changes
- **WHEN** its result no longer matches the declared behavior
- **THEN** documentation acceptance fails with a reviewable difference

#### Scenario: A user looks for coupling ratios in JSON
- **WHEN** they read the JSON section
- **THEN** it states that shared and union commit counts are published and that any ratio is derived by the consumer

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
