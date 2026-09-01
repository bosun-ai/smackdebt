## MODIFIED Requirements

### Requirement: Architecture integrates with progressive reports
Default terminal reports SHALL show architecture health totals and rated cycle
witnesses without arbitrary dependency-edge rows. A rated cycle SHALL reach a
codebase reader as one problem card carrying its existing witness, and a
package's degree SHALL reach a reader only as aggregate problem evidence such as
a fan-in or fan-out count.

No human view SHALL list dependency relations as rows at any scope, in any
mode, or at any detail level. Unresolved and ambiguous relation rows SHALL
appear only when `--all` is supplied or the selected scope is a file; at every
other scope the grouped warning sentence is their whole terminal presence.

A finding SHALL name the identity of the thing it measures, and that identity is
never a relation row. A co-change finding is about two files, so a human view
MAY write both file paths as that finding's subject — as the head of its card or
as one of its evidence lines — exactly as a cycle finding writes the file paths
of its witness and a stable-dependency finding writes its package pair. The rule
above bans the enumeration of relations nobody rated; it does not ban a rated
statement whose subject happens to be two files. Nothing else returns with the
pair: a leakage finding SHALL carry no reference count, no resolution outcome,
no relation kind, and no edge identity, so a reader can never mistake it for the
row that was deleted.

This REVERSES the previously accepted rule that `--all` and path drill show
relevant incoming, outgoing, unresolved, ambiguous, advisory, and
module-ownership relations. That rule is the root cause of the reported defect:
at a directory scope holding 296 files, "relevant" meant every edge touching the
scope, so the report printed about 1,270 unrated rows — one per import, test
imports and module wiring included — and the reader could not find a problem in
them. Relations are graph facts, not decisions; the machine report is where a
consumer reads them.

The machine report SHALL retain complete relation tables in whichever schema
version is current, so retiring one version never retires the relation tables.

#### Scenario: A repository has many ordinary uses and no cycles
- **WHEN** default terminal output is rendered
- **THEN** architecture states its health without listing arbitrary edges

#### Scenario: A user drills into one package
- **WHEN** incoming, outgoing, ownership, or advisory relations touch it
- **THEN** no relation row is printed and the relationship reaches the reader only through aggregate problem evidence

#### Scenario: A user asks for all detail at a directory
- **WHEN** `--all` is supplied at a directory scope
- **THEN** unresolved and ambiguous rows appear while no `uses` or `module_ownership` row appears

#### Scenario: A co-change finding names its two files
- **WHEN** a card states a change-leakage finding about two files
- **THEN** both repository-relative paths appear as that finding's subject and no reference count, resolution outcome, or relation kind appears beside them

#### Scenario: A Rust path uses its nearest matching module
- **WHEN** a Rust use path can match both its leaf module and one or more parent-module fallbacks
- **THEN** the first matching path level wins and broader fallbacks do not make the relation ambiguous

#### Scenario: A machine consumer reads relations
- **WHEN** the current machine report is parsed
- **THEN** every dependency edge, package edge, external dependency, resolution diagnostic, and package-graph row remains present

## ADDED Requirements

### Requirement: Propagation reach is a scoped closure
Analysis SHALL compute how far a change can propagate as a closure over the
dependency graphs it already owns, in the direction a change actually travels:
from a changed node to the nodes that depend on it, transitively. Reach SHALL be
computed by strongly connected component condensation followed by a
reverse-topological closure over fixed-width bit sets, SHALL produce integers
only, and SHALL be a descriptive fact — never rated, never a finding, never an
input to any verdict.

Three scoped forms SHALL exist, and no other:

- **Package reach at the repository root.** Each package SHALL carry the number
  of packages that transitively depend on it, counting itself. The repository's
  reach SHALL be the largest such count. It SHALL be material only when the
  repository holds at least 3 packages and that largest count is at least 2,
  because a repository with one package, or with no cross-package dependency at
  all, has nothing to say.
- **File reach inside a package.** Analysis SHALL close over every package's own
  files while the architecture graphs are built, producing one value per
  package: the largest number of files that transitively depend on one file of
  that package, counting itself. The value SHALL be material only when the
  package holds at least 20 files. A package holding more than
  `CLOSURE_NODE_LIMIT = 4_096` files SHALL be skipped and the skip SHALL be
  disclosed rather than silently absent, because the transient bit set is what
  bounds this computation.

  Every package's value SHALL be computed eagerly, never when a scope is
  rendered. The accepted rule that rendering a retained scope performs no
  project work admits no lazy closure, and a value computed on demand would make
  the same report answer differently depending on which scope a consumer asked
  for first.
- **Exact reach for a bounded candidate set.** For card evidence only, analysis
  SHALL compute the exact repository-wide reach of each file in a candidate set
  formed from the members of file dependency cycles and the files whose degree
  reaches the hub threshold, ordered by fan-in descending then by
  repository-relative path, and cut at `REACH_CANDIDATE_LIMIT = 64` files. Each
  candidate SHALL cost one reverse breadth-first search. No other file SHALL
  carry an exact reach value.

The three limits — the closure node limit, the candidate limit, and the
20-file package floor — together with the 3-package and 2-package root floors
are proposed values under review and SHALL be implemented as named integer
constants so review can move them in one place.

#### Scenario: A layered repository is analyzed
- **WHEN** fourteen packages form a layered graph whose most depended-on package is reachable from eight others
- **THEN** the repository's package reach is 9 of 14 and each package's own count remains available

#### Scenario: A repository holds one package
- **WHEN** the repository declares a single package
- **THEN** no package reach fact exists

#### Scenario: No package depends on another
- **WHEN** every package is independent
- **THEN** the largest reach is 1 and no package reach fact exists

#### Scenario: A package is too large to close over
- **WHEN** a package holds more files than the closure node limit
- **THEN** its file reach is absent and the skip is disclosed

#### Scenario: Two package scopes are rendered from one report
- **WHEN** two package scopes are rendered from the same completed report
- **THEN** each states the value computed when the report was built and rendering performs no closure, no graph traversal, and no algorithm pass

#### Scenario: A hub carries an exact reach
- **WHEN** a file is inside the candidate set and 41 files transitively depend on it
- **THEN** its exact reach value is 41 and a file outside the candidate set carries no exact value

#### Scenario: Reach is descriptive
- **WHEN** a repository's reach is large
- **THEN** no rating, finding, or verdict changes because of it

### Requirement: Core size is the largest file dependency cycle
Analysis SHALL retain the file strongly connected components the architecture
build already computes, rather than dropping them after cycle findings are
created, and SHALL expose the size of the largest component in files together
with the number of files the file dependency cycle graph is built over, which is
the parsed primary files that can carry an edge entering it.

The core SHALL be material only when it holds at least 5 files and
`core × 100 ≥ files × 2`, so a three-file cycle in a large repository and a
cycle that is a rounding error of the codebase both stay silent. Core size SHALL
be a descriptive fact: it SHALL NOT be rated, SHALL NOT create a finding, and
SHALL NOT change a verdict. The existing per-component cycle findings and their
witnesses are unchanged by it.

Both constants are proposed values under review and SHALL be implemented as
named integer constants.

#### Scenario: A large core exists
- **WHEN** the largest file component holds 34 of 210 graph files
- **THEN** the core size fact states both integers

#### Scenario: The core is a rounding error
- **WHEN** the largest component holds 3 files of 200
- **THEN** no core size fact exists, because it is below both the absolute and the proportional floor

#### Scenario: Components are retained rather than recomputed
- **WHEN** live work counters are compared before and after core size is derived
- **THEN** no additional algorithm pass, read, or Git process is recorded

### Requirement: Human graph claims require sufficient evidence
Project analysis SHALL resolve TypeScript and JavaScript aliases from the
source file's package-root `tsconfig.json` or `jsconfig.json`, with TypeScript
configuration preferred when both exist. Alias targets SHALL be relative to the
config directory. Data-only JSONC and relative inheritance inside the repository
SHALL be supported without executing configuration or loading dependency code.

A package's graph evidence SHALL be incomplete when a primary file could not be
parsed, an internal reference could not be resolved uniquely, or applicable
configuration could not be read safely. Repository evidence SHALL be complete
only when every package is complete. Root reach, core size, and hidden-coupling
absence SHALL require complete repository evidence. Package and file reach SHALL
require complete evidence for that package. One-way leakage SHALL require both
endpoint packages to be complete.

#### Scenario: Two packages use the same alias
- **WHEN** two packages each map `@/*` to their own `src/*`
- **THEN** each source resolves only through its own package configuration

#### Scenario: An internal import is unresolved
- **WHEN** a primary source contains an internal import that cannot be matched
- **THEN** the affected evidence is incomplete, the human graph claim is absent, and the machine diagnostic retains the file and target

### Requirement: A diff compares trusted graph movement
Diff analysis SHALL compare repository package reach, selected package file
reach, core size, and change-leakage findings from the current and base graphs
it already builds. Each graph side SHALL derive file presence, package
ownership, source role, and trust from that side's tree. A larger share SHALL be
worse, a smaller share SHALL
be better, and equal shares with changed integer operands or core members SHALL
be changed. Share direction SHALL use integer cross multiplication. A source
tie change with identical reach SHALL create no comparison.

Every comparison SHALL name a stable package, file, cycle anchor, or file pair.
Reach SHALL compare a named package or file with itself on both sides, using the
union of the material subjects selected by either side. A core comparison SHALL
use one file present in both sides as its anchor and compare the components that
contain that file. Disjoint largest components SHALL produce separate old and
new movements when their anchors exist on both sides; they SHALL NOT be joined
into one changed row. These comparisons SHALL reuse the reach and component data
from the existing graph builds without another graph pass.
No comparison SHALL be created when either side lacks the evidence its claim
needs. Unknown evidence SHALL never be reported as improvement.

#### Scenario: A branch increases file reach
- **WHEN** package reach changes from 12 of 26 files to 16 of 26 files with sufficient evidence on both sides
- **THEN** one worse propagation comparison names the source file and carries both fractions

#### Scenario: One graph side is incomplete
- **WHEN** a candidate movement has insufficient graph evidence before or after
- **THEN** no better, worse, or changed comparison is created and diagnostics disclose the suppression

#### Scenario: Package ownership changes in the worktree
- **WHEN** a nested package manifest exists only in the current tree
- **THEN** base graph failures belong to the package that owned the file in the base tree

#### Scenario: A modified tracked source is ignored on one side
- **WHEN** Git reports a source change but that path is excluded from one side's inventory
- **THEN** that source is absent from that side's analysis and graph evidence

#### Scenario: Base inventory metadata cannot be trusted
- **WHEN** a reachable base ignore file or name-bearing manifest cannot be read as valid text
- **THEN** diff analysis fails and does not infer file presence or package identity

#### Scenario: Different packages have the largest reach
- **WHEN** package A has the largest reach before and package B has the largest reach after
- **THEN** each comparison keeps one package identity and no row assigns package B's value to package A

#### Scenario: The largest dependency cycle changes membership
- **WHEN** the largest base cycle and largest current cycle have no files in common
- **THEN** they are compared as separate stable-anchor movements and no row joins the old cycle to the new cycle
