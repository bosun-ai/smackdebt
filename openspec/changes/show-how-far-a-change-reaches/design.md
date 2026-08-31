## Context

Smackdebt already owns both halves of the answer to "how far does a change
reach" and has never joined them. The history stream produces one change fact
per changed file per commit; the architecture build produces the file dependency
graph, the package graph, and the complete file strongly connected components,
then drops the intermediate structures. Everything this change adds is a join,
a closure, or a median over data that is already computed and already paid for.

Two orderings decide the shape of the implementation and are not negotiable:

- `load_evolution` streams history **before** `build_architecture` runs, so pair
  accumulation is graph-blind by construction. The dependency join therefore
  happens where the report is finished, over a retained pair table and a
  retained graph, as a pure function.
- The history stream hands over one commit at a time and retains no per-commit
  file list, so pairs and amplification histograms must be accumulated inline
  during that one stream. There is no second pass over history to add later.

The constraint that shapes everything else is that live work counts are
load-bearing evidence: acceptance and the performance report check assert exact
inventory, read, Git process, parser visit, and algorithm pass totals per flow.
Every computation here rides an existing pass and records no new one.

> **Constants under review**
>
> Every constant below is a proposed value awaiting review during
> implementation; each is a named integer constant so review can move it in one
> place, and each is calibrated against real repositories in the final slice
> before it is frozen:
>
> - Pair guards (slice 3): `BULK_COMMIT_FILES = 25`,
>   `RETAINED_FILE_PAIR_SHARED_COMMITS = 3`,
>   `RETAINED_FILE_PAIR_SIMILARITY_PERMILLE = 100`,
>   `RETAINED_FILE_PAIR_LIMIT = 1_000_000`.
> - Leakage detectors (slice 5): `LEAKAGE_MIN_DISTANCE = 2`,
>   `LEAKAGE_SHARED_COMMITS = 5`,
>   `required_permille(d) = max(400 - (d - 2) * 50, 200)`,
>   `PATH_PROBE_NODES = 4_096`.
> - Amplification (slice 4): `AMPLIFICATION_MAX_FILES = 1_000`,
>   `AMPLIFICATION_MIN_COMMITS = 10`, `AMPLIFICATION_MIN_MEDIAN = 3`.
> - Propagation and core (slice 2): root reach needs at least 3 packages and a
>   largest reach of at least 2; package reach needs at least 20 files;
>   `CLOSURE_NODE_LIMIT = 4_096`; `REACH_CANDIDATE_LIMIT = 64`; core size needs
>   at least 5 files and `core * 100 >= files * 2`.
>
> The accepted `problem-clustering` thresholds, the `hotspot-analysis` minimum
> touch count, the `verdict-policy` tier terms, and the
> `progressive-exploration` budget and ladder are not reopened here.

## Goals / Non-Goals

**Goals:**

- Answer "how far does a change here reach" with facts the report already owns,
  at no new I/O cost.
- Name the two co-change shapes that a dependency graph alone cannot see, and
  name them only when the evidence is strong enough to survive review.
- Give the verdict head three system numbers that make an architecture argument
  without pretending to rate one.
- Keep the default view about the same size it is today: at most one or two new
  items on real repositories.

**Non-Goals:**

- No new schema version. Version 4 gains members additively.
- No gate signal, no baseline movement, and no change to the ratcheted set.
- No diff-mode surface: no comparison, no debt-diff member, no diff verdict
  input.
- No module-depth or layering model. Directory distance is the only structural
  metric introduced, and it is an integer over the paths discovery already owns.
- No per-file change histograms and no per-file amplification.
- No new rating, tier input, or verdict escalation.

## Decisions

### The change graph is not the finding graph

`HistoryChangeFact::affects_findings()` admits primary, test, example, and
benchmark roles. That is right for churn and for package coupling — a test
commit is real activity, and a test-role dependency genuinely explains why two
packages move together — and wrong for file co-change. A test file and its
subject change in the same commit by construction; admitting the test role would
give every well-tested file a perfect co-change partner and rank good practice
as leakage.

So a new predicate, `HistoryChangeFact::enters_change_graph()` = primary role
and trusted parse, named to mirror `DependencyEdge::enters_verdict_graph()`. The
naming is the point: a reader who knows one predicate can guess the other, and
the two graphs a verdict is built from are now both named after what they admit.

### Pairs are accumulated blind and joined late

Pair accumulation runs as a new member of `EvolutionAccumulator::accept`, beside
churn, coupling, and concentration, because that is the only place a commit's
file list exists. It knows nothing about dependencies, so it retains every
cross-directory pair that clears the retention floor and lets the join at
`CodebaseReportBuilder::finish` decide which pairs mean something.

Retention floors are deliberately looser than detector floors: three shared
commits and 10% agreement retain a pair; a finding needs five shared commits and
between 20% and 40% agreement depending on distance. The gap is where a future
detector, a JSON consumer, and the calibration task all live. It is also why the
below-floor pairs must stay out of every human view — the retained population is
a research surface, not a report.

The accumulator keeps its own per-file commit count over exactly the commits it
accumulated pairs from, and derives `union = touches(a) + touches(b) - shared`
from that. Reusing the file history touch count would mix populations: a bulk
commit is excluded from pairs but counted in file history, so union would grow
where shared cannot, and similarity would drift down for exactly the files that
appear in sweeping commits.

### One finding table, two kinds

`ChangeLeakageFinding { kind, coupling, interface: Option<FileId> }` follows
`ArchitectureFinding`, where `package_cycle` and `file_cycle` already share a
table and a kind discriminator. Two tables would add two entries to the claim
vocabulary, two evidence kinds, two schema tables, and two audit registrations
to say one thing: this pair of files is coupled in a way the graph does not
explain. One table with a kind adds one of each.

Findings are ordered by kind, then distance descending, then shared commits
descending, then the two file identities. Distance is the severity signal, so it
belongs in the order a reader of the table sees. It deliberately does **not**
become a key of the problem rank: the accepted rank ends in anchor path and
anchor start line, and inserting a distance key before them would change the
order of existing cards for a reason that has nothing to do with them.

### Distance buys severity by lowering the bar

`required_permille(d) = max(400 - (d - 2) * 50, 200)`. Two directories apart
needs 40% agreement, six or more needs 20%. Two files in sibling folders that
change together are often just one feature; two files five directories apart
that change together at all are evidence that a boundary is not holding. The
rule is integer-only, monotone, and floored, so it can be reasoned about without
a table of cases — and the floor at 20% keeps it from degenerating into "any
distant pair that ever changed together".

### Absence is proved, never inferred

A `hidden_coupling` finding claims that no dependency path explains the
co-change. That is a strong claim, and the only honest way to make it is to
prove it — against one graph, for both stages.

That graph is the **connection graph**: every `uses` and every
`module_ownership` relation between primary trusted files. It is wider than the
file dependency cycle graph the leaky-interface rule reads, deliberately. The
cycle graph drops the wiring between a Rust parent and the child it declares,
which is right when asking whether a production import propagates change and
catastrophic when asking whether any dependency exists at all: an owning pair
that imports each other would be reported as changing together *without a
dependency*, which is the one thing the card must never say. The two rules read
two graphs, and neither admission rule may be substituted for the other.

Stage one is the package projection of that same graph: if the two files live in
different packages and the package connection matrix shows neither package
reaches the other, no file-level connection can exist. Building the matrix over
the connection graph rather than over the verdict-graph package edges is what
makes a stage-one *separate* answer decisive — a verdict-graph matrix is a
subset, so its "separate" could be overturned by an ownership edge the wider
graph admits, and every stage-one hit would have to fall through to the probes
anyway. The cost is one extra closure over the package graph, which has tens of
nodes where the file graph has thousands. Because that projection is symmetric,
its condensation has no edge between components and the matrix is one component
label per package rather than a bit set per package: two packages reach each
other exactly when their labels agree.

Stage two is a budgeted breadth-first walk over the connection graph, bounded at
`PATH_PROBE_NODES`, from one end of the pair and — only when that walk exhausts
its budget — from the other. Either walk proves the whole claim, because the
graph holds both directions of travel: exhausting everything that reaches one
end without meeting the other proves absence both ways. What the two walks do
not share is cost, because each explores one file's own side of the graph, so an
undecided answer says only that the side it started from is large. Asking the
other end decides every pair whose smaller side fits the budget and makes the
answer independent of which file the walk started from.

The rule was first written as one walk per direction with both required to
answer separate, which is the same proof read the wrong way round: it decides
only pairs whose *larger* side fits the budget and withholds findings the first
walk has already proved. A pair is therefore undecided only when both of its
sides exceed the budget, and the worst case is still two walks.

The alternative — treat "no path found within budget" as "no path" — would make
the strongest claim in the product on the weakest evidence, and it would make
findings depend on graph size in a way no reader could see. Silence is the right
failure mode: a repository with one enormous package will report fewer hidden
pairs than it might contain, and that is a disclosed limitation rather than a
wrong statement.

### Hybrid surfacing: evidence first, card only if nothing else names it

The budget names each problem once. A file that already has a card does not need
a second one to carry two more numbers, so leakage findings that belong to a
carded file become evidence lines on that card, and a standalone card exists
only where no card names the subject. That keeps the number of default items
governed by the number of *problems*, not by the number of *signals*.

For a pair, the claim goes to the lower-indexed file when that file already has
a card. The alternative — either endpoint may claim — was rejected because it
makes the location of the pair's evidence depend on which file happened to carry
unrelated debt, so the same pair moves between cards as the rest of the report
changes. A fixed endpoint is data-stable and explainable; the cost is that a
pair whose *higher*-indexed file has a card still gets a standalone card, which
is the shape a reader can predict from the rule.

### The system numbers are stated, never rated

Reach, core size, and amplification join the verdict head as analysis-owned
sentences beside the share and the coverage qualifier, and they never move a
tier. Two reasons. First, they are not debt: a large reach in a layered
monolith may be exactly the intended architecture, and a tier that dropped
because of it would be wrong in a way the user cannot argue with. Second, two of
the three derive from history, and the tier must not move with wall-clock time —
the same rule that keeps history signals out of the gate.

Absence is a first-class outcome. A one-package repository states no reach; a
three-file core in a two-hundred-file repository states nothing; a scope with
nine commits states no amplification. A hedged sentence would be worse than
silence, because a reader cannot tell a weak number from a strong one once it is
printed.

### Reach is scoped, because exact global reach is not affordable or useful

Exact per-file reach for every file is a closure per node. Instead:

- The **package graph** is small enough to close over completely, so the root
  sentence is exact.
- **Every package's** files are closed over eagerly inside the architecture
  build, one package at a time and each bounded by `CLOSURE_NODE_LIMIT`, so the
  report holds one value per package before any scope is rendered. Lazy was
  considered and rejected: the accepted rule that rendering a retained scope
  performs no project work leaves no room for a closure behind a getter, and a
  cached-on-first-read value would make two consumers of one report do different
  amounts of work. The cost is bounded by construction — each package closes
  over its own files only, so the aggregate is the sum of per-package work, and
  one transient bit set is live at a time.
- **Card evidence** needs exact reach only for the files a card already names,
  so a bounded candidate set — cycle members and hub-degree files, cut at
  `REACH_CANDIDATE_LIMIT` — gets one reverse search each.

This is the same shape as the accepted hotspot rule: compute the expensive fact
only where a reader will see it — but compute it while the report is built, not
while it is read.

### Amplification lives at directory scope and up

The median of files-per-commit is a property of a region, not of a file. A
per-file histogram would be a handful of observations for most files, and a
median of three observations is noise dressed as a fact. Directory-and-up also
matches how the number is read: "a typical change here touches 4 files" answers
a question about an area of the codebase.

A commit contributes one observation to each directory it touched and to each
ancestor of those directories, so a commit is counted once per directory rather
than once per file — otherwise a commit touching ten files in one directory
would count ten times there and once elsewhere, and the median would measure
commit shape rather than region behavior.

### The gate does not move this round, and the future row is written down

No signal is added. `core_size` is the only new fact that is time-invariant and
therefore *could* be ratcheted, and it is deliberately left out while its
constants are under review: ratcheting a number whose definition may move would
force a baseline update in the slice that moved it, which is exactly the churn
the gate exists to prevent.

The future shape, recorded here so a later change does not have to rediscover
it and **not** proposed as accepted behavior now:

- a `GateSignal::CoreSize` variant declared between `ContainerSize` and
  `Cyclomatic`, because the variants are declared in the byte order of their
  identifiers and `core_size` sorts there;
- one row per report, `.` for the path, `core_size` for the signal, `0` for the
  High column, and the file count of the largest file strongly connected
  component for the Watch column, because a core is a Watch-shaped fact
  attributed to the repository rather than to a file — written in the baseline's
  tab-separated form as
  `.<TAB>core_size<TAB>0<TAB><files in largest file SCC>`;
- a single `gate --update` in the slice that introduces it, with the baseline
  diff reviewed like any other deliberate debt.

### The README ban ends because the phrase came back as a name

`change together without a dependency` is currently asserted **absent** from the
README. The assertion was correct: the phrase belonged to a coupling section
that was deleted, and the ban proved it had not crept back. The phrase is now
the human name of a frozen pattern id, so the documentation must contain it. The
slice that adds the name deletes the assertion and records the reason where the
assertion was written, so a future reader can tell a sanctioned revival from a
regression.

### Additive v4, not v5

Four tables, one package-graph member, three verdict members, two pattern ids,
and two evidence kinds are added to version 4. Nothing is removed and no member
changes meaning, which is the accepted test for staying on version 4 — the same
test `earn-the-verdict` and `make-problems-legible` applied. The schema uses
`additionalProperties: false` with `required` lists, so each schema edit lands in
the same slice as its serializer.

## Risks / Trade-offs

- **Calibration is the whole game.** `LEAKAGE_SHARED_COMMITS = 5` against a
  90-day window on a quiet repository may produce zero findings on smackdebt
  itself. That is a legitimate outcome, not a bug: the evidence stays fixture-
  driven, and the calibration task records the real card sets at two window
  lengths before the constants are frozen.
- **Window sensitivity.** Every co-change number moves with the selected window.
  The floors are fixed for now and revisited after calibration; the alternative,
  scaling floors by window length, would make two runs of the same repository
  incomparable.
- **One-package repositories lose stage-one filtering.** Every hidden-coupling
  candidate then depends on the budgeted probes, which both costs more and
  reports less. A fixture covers the shape and the `evolution-wide` profile
  measures it.
- **Double renames under-report pairs.** History joins a current file only
  through a parsed rename chain, so a broken chain silently drops pairs. It is
  disclosed by the existing rename-gap warning and documented rather than fixed
  here.
- **The pair table limit is a disclosed approximation.** Beyond the limit, new
  keys are refused rather than evicted, so the retained population is biased
  toward pairs seen earlier in the stream. Measured on `evolution-wide` and on a
  private repository before the limit is frozen.
- **Four snapshot-churn events.** Terminal snapshots move in slices 2 and 5;
  JSON snapshots move in slices 2, 3, 4, and 5. Each lands on an otherwise quiet
  baseline and each regenerated snapshot is reviewed per case.
- **The gate must not move while new modules land.** New analysis modules can
  trip the file-size and container-size thresholds the repository ratchets
  against itself. The response is to restructure the module, not to update the
  baseline.

## Migration Plan

1. Author and strict-validate this change (slice 0).
2. Pure primitives (slice 1): the directory tree, the reachability module, and
   the shared nearest-rank median moved out of the problem module. No report
   surface changes, so the analysis API snapshot is the only regenerated
   artifact.
3. Core size and propagation reach (slice 2): retain what the architecture build
   already computes, add the closures, the candidate reach, the two verdict
   sentences, and their JSON and schema members. Terminal and JSON snapshots
   churn once, together, on a quiet baseline.
4. The file co-change accumulator (slice 3): the change-graph predicate, the
   accumulator, its guards, the coverage disclosure, and the JSON table. No
   terminal surface, so JSON snapshots churn alone.
5. Change amplification (slice 4): histograms, the scope join, the head
   sentence, and its JSON member.
6. Leakage detectors and cards (slice 5): the join, the finding table, the two
   patterns at the tail of the enum, hybrid claiming, the evidence wordings, the
   `shotgun_pair` rename, the README ban-list deletion, and the extended audits.
   This is the slice that moves the terminal.
7. Calibration and documentation close-out (slice 6): the release binary over
   real repositories at two window lengths, constants frozen or adjusted with
   the card sets recorded, then README, `ARCHITECTURE.md`, and the performance
   digest staleness note.

Rollback is per-slice revert throughout. No slice writes a committed baseline or
a stored artifact that a revert would leave behind.
