## ADDED Requirements

### Requirement: Change leakage joins co-change with the dependency graph
Analysis SHALL produce change-leakage findings by joining retained file change
coupling with the file dependency graph after both exist. The join SHALL be a
pure function of a borrowed pair table and a borrowed graph, SHALL run once
while the report is finished, and SHALL NOT read a file, walk the filesystem,
start a Git process, visit a parser, or record an algorithm pass. It creates no
measurement and rates no unit: it decides which already-measured pairs are worth
naming.

Both detectors SHALL write into one finding table whose rows carry a kind of
either `leaky_interface` or `hidden_coupling`, the coupling pair they were
decided from, and — for `leaky_interface` — the identity of the file whose
importers follow it. One table with a kind follows the accepted
`ArchitectureFinding` precedent, where `package_cycle` and `file_cycle` share a
table, and it keeps the claim and evidence vocabularies growing by one family
rather than by two. Every finding SHALL be rated Watch.

The table SHALL be ordered by kind, then directory distance descending, then
shared commits descending, then the left and then the right file identity, which
discovery orders by repository-relative path. Distance orders the finding table
and SHALL NOT become a key of the problem rank, whose accepted keys end in
anchor path and anchor start line.

#### Scenario: The join is observed as work
- **WHEN** live work counters are compared immediately before and after the report is finished
- **THEN** the join adds no inventory visit, read, Git process, parser visit, or algorithm pass

#### Scenario: A detector is tested alone
- **WHEN** the join is exercised over a hand-built pair table and a hand-built graph
- **THEN** it produces its findings without composing a report

#### Scenario: Two findings tie on distance
- **WHEN** two findings of the same kind share a directory distance and a shared-commit count
- **THEN** the left and then the right file identity produce one stable order in serial and parallel runs

### Requirement: The change graph admits primary trusted source only
A history change fact SHALL enter file pair accumulation and change
amplification only when its source role is primary and its parse trust is
trusted. Analysis SHALL express this as its own predicate on the change fact,
named for the change graph the way `DependencyEdge::enters_verdict_graph` is
named for the verdict graph, and SHALL NOT reuse the existing
history predicate that decides whether a change affects evolutionary findings.

That existing predicate admits test, example, and benchmark roles, which is
correct for churn and for package coupling and wrong here: a test file and its
subject change in the same commit by construction, so admitting the test role
would manufacture the strongest possible co-change pair for every
well-tested file and name good practice as leakage. Fixture, generated,
recovered, and failed source SHALL likewise contribute no pair and no
amplification observation, and SHALL remain complete in the machine report as
history context.

#### Scenario: A test file changes with its subject
- **WHEN** a test file and the primary file it exercises change together in twenty commits
- **THEN** no file pair is accumulated for them and no leakage finding names them

#### Scenario: A generated file changes with its source
- **WHEN** a generated file and a primary file change together
- **THEN** the pair is never accumulated and both files keep their existing churn and history rows

#### Scenario: Two primary trusted files change together
- **WHEN** two parsed primary files in different directories change in the same commit
- **THEN** the commit contributes one shared change to that pair

### Requirement: File pair accumulation is guarded and disclosed
Pair accumulation SHALL run inside the single streamed history process, one
commit at a time, because per-commit file lists are not retained. For each
commit, analysis SHALL take the distinct files whose change facts enter the
change graph and SHALL apply these guards:

- **Bulk commits.** When that count exceeds `BULK_COMMIT_FILES = 25`, the commit
  SHALL contribute no pair at all and SHALL be counted as a bulk commit. A
  sweeping rename, a formatting pass, or a dependency bump would otherwise
  create a quadratic burst of pairs that say nothing about design. Churn,
  touches, package change coupling, contributor concentration, hotspots, and
  change amplification SHALL be unaffected by this guard.
- **Same-directory pairs.** A pair whose two files share a directory SHALL never
  be stored, because co-change inside one directory is what a directory is for.
- **Retention.** After the stream ends, a pair SHALL be retained only when its
  shared commits reach `RETAINED_FILE_PAIR_SHARED_COMMITS = 3` and
  `shared × 1000 ≥ union × RETAINED_FILE_PAIR_SIMILARITY_PERMILLE`, with
  `RETAINED_FILE_PAIR_SIMILARITY_PERMILLE = 100`.
- **Storage limit.** When the accumulator already holds
  `RETAINED_FILE_PAIR_LIMIT = 1_000_000` keys, no new key SHALL be created and each
  declined pair SHALL be counted. Existing keys SHALL keep accumulating, so the
  approximation only ever loses pairs it had not yet seen.

The accumulator SHALL keep its own per-file commit count over exactly the
commits from which pairs were accumulated, and SHALL derive
`union_commits = touches(left) + touches(right) − shared_commits` from it, so the
shared and union operands describe one population. That count SHALL be separate
from the file history touch count, which stays over every eligible commit.

History coverage SHALL disclose the bulk commit count and the declined pair
count through its own builder, mirroring the accepted window builder, so an
approximation is always stated rather than silent. Every stored value SHALL be
an integer.

The four guard constants are proposed values under review and SHALL be
implemented as named integer constants so review can move them in one place.

#### Scenario: One commit rewrites thirty files
- **WHEN** a commit changes thirty files that enter the change graph
- **THEN** no pair is accumulated from it, the bulk commit count is one, and every churn, touch, package coupling, and amplification value is what it would have been without the guard

#### Scenario: Two files in one directory change together
- **WHEN** two primary files of the same directory change in eight commits
- **THEN** no pair exists for them at any similarity

#### Scenario: A pair is below the retention floor
- **WHEN** a cross-directory pair has 2 shared commits, or a shared-to-union ratio below one tenth
- **THEN** it is not retained

#### Scenario: The pair table reaches its limit
- **WHEN** accumulation would create a key beyond the limit
- **THEN** the key is not created, the declined pair count rises, and history coverage discloses it

### Requirement: A retained pair is not a finding
Retention and finding creation SHALL use different thresholds and SHALL be kept
distinct. A retained pair is the population a detector reads; a finding is a
statement about design. A retained pair that no detector turned into a finding
SHALL reach the machine report and SHALL NOT reach any human view, including
`--all` and a selected file scope, and SHALL NOT reach a problem card.

This is deliberately stricter than the accepted rule for weak package coupling,
which stays visible under `--all`. A file pair table is one to two orders of
magnitude larger than a package pair table, so the same visibility would flood
the very views this product exists to keep short.

#### Scenario: A retained pair produces no finding
- **WHEN** a cross-directory pair is retained but meets no detector's thresholds
- **THEN** it appears in the machine report and appears in no default view, no `--all` view, no file-scope view, and no card

#### Scenario: A retained pair is counted
- **WHEN** the machine report is read
- **THEN** every retained pair carries its two file indexes, shared commits, union commits, and directory distance as integers

### Requirement: Directory distance is an integer tree fact
Analysis SHALL own a directory tree over the repository-relative paths discovery
already produced, built once before history streaming begins, in time
proportional to the total number of path components. It SHALL expose each file's
directory, each directory's parent and depth, and the integer distance
`distance(a, b) = depth(a) + depth(b) − 2 × depth(lowest common ancestor)`.

Distance SHALL be produced and compared as an integer, SHALL be 0 for two files
of the same directory, and SHALL be 1 when one directory contains the other
directly. Distance SHALL be a property of the pair alone, so it never depends on
the selected scope.

#### Scenario: Two files share a directory
- **WHEN** the distance between two files of the same directory is read
- **THEN** it is 0

#### Scenario: Two sibling directories are compared
- **WHEN** two files sit in sibling directories under one parent
- **THEN** their distance is 2

#### Scenario: Two subtrees meet at the repository root
- **WHEN** two files sit three and two directories below the root in different subtrees
- **THEN** their distance is 5 and no path string is compared to compute it

### Requirement: A leaky interface is named from its importers
Analysis SHALL create a `leaky_interface` finding for a retained pair `(a, b)`
when every one of these holds:

- `b` depends on `a` through an edge that enters the file dependency cycle
  graph, and `a` does not depend on `b` through one. The direction is what the
  pattern is about: a *follower* changing with the *interface* it imports is the
  observable signature of an interface that leaks its internals. The cycle
  graph's admission rule is reused because it already excludes module-ownership
  wiring, so a Rust parent and the child it declares — which change together by
  construction — can never be named. This rule and the hidden-coupling rule
  therefore read deliberately different graphs: this one wants a production
  import whose changes follow through it, so ownership wiring is excluded, while
  hidden coupling claims no dependency of any kind exists and therefore reads
  the wider connection graph defined below. Neither admission rule SHALL be
  substituted for the other.
- `distance(a, b) ≥ LEAKAGE_MIN_DISTANCE = 2`.
- `shared_commits ≥ LEAKAGE_SHARED_COMMITS = 5`.
- `shared × 1000 ≥ union × required_permille(distance)`, where
  `required_permille(d) = max(400 − (d − 2) × 50, 200)`. Distance buys severity
  by lowering the similarity bar: two directories apart requires 40%, three
  requires 35%, and six or more requires 20%. Two files far apart that change
  together at all is a stronger statement than two neighbours that change
  together often.

Exactly one finding SHALL exist per ordered `(interface, follower)` pair. When
two files import each other, neither direction SHALL produce a finding, because
a mutual dependency is a cycle and the cycle finding already names it.

The two named floors and the three terms of `required_permille` — the 400
permille base, the 50 permille step, and the 200 permille floor — are proposed
values under review and SHALL be implemented as named integer constants so
review can move them in one place.

#### Scenario: An importer follows its interface across three directories
- **WHEN** `b` imports `a`, they sit three directories apart, and they share 7 of 12 commits
- **THEN** one `leaky_interface` finding names `a` as the interface and `b` as the follower

#### Scenario: The interface imports its follower
- **WHEN** the only edge runs from `a` to `b` rather than from `b` to `a`
- **THEN** no `leaky_interface` finding names `a`

#### Scenario: The two files are one directory apart
- **WHEN** an importer and its target sit in a directory and its direct parent
- **THEN** no finding is created because the distance is below the minimum

#### Scenario: Support is one commit short
- **WHEN** an importer and its target two directories apart share 4 commits at any similarity
- **THEN** no finding is created

#### Scenario: Distance lowers the bar
- **WHEN** one pair two directories apart shares 30% of its commits and another six directories apart shares 25% of its commits
- **THEN** only the second is a finding

#### Scenario: A Rust module owns its child
- **WHEN** a Rust file declares a child module and the two change together often
- **THEN** the ownership pair is outside the cycle graph and no finding is created

### Requirement: Hidden coupling proves absence, never infers it
Absence SHALL be proved against one graph, defined here and used by both stages.
The **connection graph** SHALL admit every `uses` relation and every
`module_ownership` relation whose two endpoint files are primary-role and
trusted, in both directions of travel.

The connection graph is deliberately wider than the file dependency cycle graph
the leaky-interface rule reads, and the contrast is the point. That rule is
about a production import whose changes still follow through it, so it excludes
the wiring a Rust parent and the child it declares share. This rule claims that
*no code dependency of any kind* explains the co-change, so a pair joined by a
module declaration, or by the imports that accompany one, is connected and SHALL
never be named. Reading the cycle graph here would mint a
`change together without a dependency` card for exactly the pair that owns
itself.

Analysis SHALL create a `hidden_coupling` finding for a retained pair `(a, b)`
when the distance, support, and distance-scaled similarity gates of the leaky
interface rule all hold **and** no path connects the two files in either
direction over the connection graph. Absence SHALL be proved in two stages:

1. When the two files belong to different packages and the package connection
   matrix — the transitive closure of the package-level projection of the
   connection graph — shows that neither package reaches the other, the files
   are separate and the finding is created. The matrix SHALL be built over the
   connection graph and SHALL NOT be a verdict-graph closure, so a stage-one
   *separate* answer is decisive rather than a shortcut a wider graph could
   overturn. It SHALL be derived once with the condensation machinery the
   propagation closures already use, and SHALL be distinct from the
   propagation-reach matrix, which answers a different question over a narrower
   graph.
2. Otherwise, analysis SHALL run one budgeted reverse breadth-first search per
   direction over the connection graph, each visiting at most
   `PATH_PROBE_NODES = 4_096` nodes. A probe SHALL answer *reaches*, *separate*,
   or *undecided*, and SHALL answer undecided exactly when it exhausted its
   budget without settling the question.

An undecided probe SHALL produce no finding. A `hidden_coupling` finding claims
that no dependency explains the co-change, and a claim that strong SHALL NOT
rest on a search that ran out of budget. The two stages SHALL agree: stage one
SHALL only ever conclude separate, never reaches.

`PATH_PROBE_NODES` is a proposed value under review and SHALL be implemented as
a named integer constant.

#### Scenario: Two packages cannot reach each other
- **WHEN** a qualifying pair lies in two packages with no entry in either direction of the package connection matrix
- **THEN** one `hidden_coupling` finding is created without running a file-level probe

#### Scenario: A path exists inside one package
- **WHEN** a qualifying pair lies in one package and a probe finds a path from one file to the other
- **THEN** no finding is created

#### Scenario: An owning pair changes together
- **WHEN** a qualifying pair is joined only by a Rust module declaration and the imports that accompany it, which the file dependency cycle graph excludes
- **THEN** the connection graph admits those relations, the pair is connected, and no `hidden_coupling` finding is created

#### Scenario: Two packages are joined only by module ownership
- **WHEN** the only relation between two packages is a module-ownership relation and a qualifying pair spans them
- **THEN** the package connection matrix reports the two packages connected, the pair falls through to the probes, and no finding is created on the strength of the package stage

#### Scenario: A probe exhausts its budget
- **WHEN** a probe visits its whole node budget without finding a path or exhausting the reachable set
- **THEN** the answer is undecided and no finding is created

#### Scenario: One package holds the whole repository
- **WHEN** a repository declares a single package so the package stage can never separate a pair
- **THEN** every qualifying pair is decided by the file-level probes alone and an undecided probe still creates no finding

### Requirement: Change amplification is a median of files per commit
Analysis SHALL accumulate, during the one history stream, a sparse per-directory
histogram of how many files each commit touched. For a commit, the observation
value SHALL be the number of distinct files repository-wide whose change facts
enter the change graph, clamped at `AMPLIFICATION_MAX_FILES = 1_000` so a
repository-wide sweep cannot unbound the key space. A commit SHALL add exactly
one observation to each directory that owns one of its changed files and to each
ancestor of those directories, so a commit is counted once per directory and
never once per file.

A scope's amplification SHALL be the nearest-rank median of its directory's
histogram, which is an integer and needs no interpolation. The nearest-rank
median SHALL be one shared implementation used by every caller that needs one.

Every scope SHALL map to exactly one directory: the repository scope to the root
directory, a package scope to that package's root directory, and a directory
scope to itself. The mapping SHALL be stated rather than inferred, because a
package whose root directory is the repository root reads the root histogram,
which is the same fact stated at two scopes rather than two different numbers.

Amplification SHALL exist for the repository, package, and directory scopes only,
and SHALL be absent at a file scope, where a per-file histogram would state
sample noise as a fact. It SHALL be material only when the scope's histogram
holds at least `AMPLIFICATION_MIN_COMMITS = 10` observations, its median is at
least `AMPLIFICATION_MIN_MEDIAN = 3`, and the history stream is complete; below
any of those it SHALL be absent rather than stated weakly.

The clamp and the two materiality constants are proposed values under review and
SHALL be implemented as named integer constants.

#### Scenario: A commit touches four files in two directories
- **WHEN** one commit changes four change-graph files spread over two directories
- **THEN** each of those directories and each of their ancestors receives exactly one observation of value 4

#### Scenario: A median is computed
- **WHEN** a directory's histogram holds the observations 2, 3, 3, 4, and 9
- **THEN** its amplification is the nearest-rank median 3

#### Scenario: A package rooted at the repository root is selected
- **WHEN** a package whose root directory is the repository root is selected
- **THEN** it reads the root directory's histogram and states the same median the repository scope states

#### Scenario: The scope has too little history
- **WHEN** a directory's histogram holds 9 observations
- **THEN** the scope has no amplification fact rather than a weak one

#### Scenario: History is incomplete
- **WHEN** the history stream is shallow or unavailable
- **THEN** no scope carries an amplification fact

#### Scenario: A file scope is selected
- **WHEN** the selected scope is a file
- **THEN** no amplification fact exists for it

### Requirement: Change leakage states facts and never moves a verdict
No change-leakage finding and no amplification fact SHALL change a tier, the
counts behind it, the worst offender, a health rating, a size rating, or an
architecture finding. They SHALL NOT enter a debt-diff selection, a comparison,
or a diff verdict, so diff output is unchanged by this capability. They SHALL
NOT enter the ratchet gate, which excludes history-derived signals by rule.

A change-leakage finding SHALL be claimable by problem clustering exactly as an
existing finding family is, and its rating SHALL be the Watch rating the finding
carries, so the vocabulary a reader already knows keeps its meaning.

#### Scenario: The same tree is analyzed with and without history
- **WHEN** one tree is analyzed with history available and with history unavailable
- **THEN** the tier, its counts, the worst offender, and every rated finding are identical and only the history-derived facts differ

#### Scenario: A worktree diff is taken
- **WHEN** a diff runs over a tree whose codebase report carries leakage findings
- **THEN** the diff verdict, its per-family counts, and its selection are what they would be without the capability
