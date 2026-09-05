# Smackdebt

**Find the code that hurts, then see whether your change made it better.**

Smackdebt turns source metrics and Git history into two short reports:

- How bad is this codebase, and where are the hotspots?
- How did this worktree change compared with a Git ref?

Run it without configuration. Start at the repository summary, then pass one of
the reported paths to inspect a package, directory, file, or function.

Every report says its answer out loud in words, so the same bytes serve a human
at a terminal and a script or a model reading a pipe.

## Install

```console
cargo install --path crates/cli
```

Smackdebt runs locally and does not upload source code or analysis data.

## Check a codebase

Run `smackdebt` from anywhere inside a repository. Every terminal example below
is real output from the released binary reading Smackdebt's own repository, so
the numbers are a snapshot and move as the code does:

```console
$ smackdebt

smackdebt · repository root
  Worn in the usual places.
20 high · 63 watch · 3,305 checked
worst: crates/project/src/project.rs — hot AND complex

AREAS
  scripts · 5 high · 10 watch
  crates/analysis · 4 high · 17 watch
  crates/cli · 4 high · 3 watch
  crates/languages · 3 high · 9 watch
  crates/project · 2 high · 21 watch

PROBLEMS
  high does too much · crates/project/src/project.rs
  high hot and complex · crates/languages/src/dependency.rs
  high hot and complex · crates/output/src/output.rs
  high hot and complex · crates/cli/tests/unified_acceptance.rs
  high hot and complex · crates/analysis/src/evolution.rs
  high hot and complex · crates/cli/src/app.rs
  high hot and complex · crates/cli/tests/acceptance.rs
  high hot and complex · crates/languages/tests/language_fixtures.rs
  high everything depends on this · crates/analysis/src/report.rs
  high hot and complex · scripts/performance/review-workloads.py
  high ArchitectureGraph::new · method · crates/analysis/src/architecture.rs:17
  high generic_source_roles · function · crates/discovery/src/inventory.rs:228
  high compare_units · function · crates/analysis/src/comparison.rs:123
  high generate · function · scripts/performance/workload.py:262
  high main · function · scripts/performance/check-report.py:156
  high validate · function · scripts/performance/check-workload-reviews.py:71
  high cycle_witness · function · crates/analysis/src/cycle_witness.rs:3
  high violations · function · scripts/check-entry-modules.py:30
  high strongly_connected_components · function · crates/analysis/src/
        strongly_connected_components.rs:1
  high glob_matches · function · crates/discovery/src/glob.rs:6
  watch vue.rs · closure · crates/languages/src/vue.rs:97
  watch language_slot · function · crates/languages/src/engine.rs:168
  watch CodebaseRequest::with_thresholds · method · crates/project/src/requests.rs:165
  watch circular dependency · crates/analysis/src/change_coupling.rs

WARNINGS
  warning 3 imports could not be followed
        2 named nothing in the repository · 1 matched more than one file
  warning 1 source file could not be fully parsed.

  next: smackdebt scripts
```

The report opens with a verdict block: the selected scope, the sentence for its
tier, the counts behind that verdict with every count labeled by its word, and
the single worst thing with its repository-relative path and the reason it is
worst. Architecture evidence appears only beside the package, file, cycle, or
package pair it describes.

The codebase tiers are fixed. Their identifiers are the stable vocabulary for an
integration; their sentences are what a person reads:

| Tier | Sentence |
| --- | --- |
| `empty` | Nothing was checked. |
| `clean` | Clean. Ship it. |
| `solid` | Solid, with rough edges. |
| `worn` | Worn in the usual places. |
| `fights_back` | This code fights back. |
| `lost` | The code is winning. |

Smackdebt combines fixed code-health limits with recent activity. A complex
function that changes often rises to the top and says `hot (n commits)`. A
complex function in quiet code still appears as debt, but does not outrank
active risks just because it is large.

`AREAS` appears when debt spans several child paths and shows at most five. The
first path is the next useful place to inspect, and `next:` names the exact
command. `PROBLEMS` is the debt itself, worst first. Empty `AREAS`, `PROBLEMS`,
and `WARNINGS` sections stay out of the way.

`PROBLEMS` names problems instead of listing measurements. One card groups the
findings the report already produced around one file, one cycle, one package, or
one package pair, so a file carrying twenty findings is named once rather than
twenty times. [Read the problems](#read-the-problems) lists the pattern names,
the evidence each one needs, and the thresholds behind them.

Pass a reported path to progressively inspect the next level. Repository,
package, directory, and file scopes retain the same repository-relative paths:

```console
$ smackdebt crates/analysis

smackdebt · crates/analysis
  Worn in the usual places.
  3 of the repository's 18 high live here.
  A typical change here touches 4 files.
3 high · 17 watch · 1,585 checked
worst: crates/analysis/src/evolution.rs — hot AND complex

PROBLEMS
  high change spreads far · crates/analysis/src/evolution.rs
  high everything depends on this · crates/analysis/src/report.rs
  high ArchitectureGraph::new · method · crates/analysis/src/architecture.rs:17
  high cycle_witness · function · crates/analysis/src/cycle_witness.rs:3
  high strongly_connected_components · function · crates/analysis/src/
        strongly_connected_components.rs:1
  watch change spreads far · crates/analysis/src/comparison.rs
  watch GateSnapshot::from_report · method · crates/analysis/src/gate.rs:150
  watch OrphanCandidate<'a>::new · method · crates/analysis/src/orphan.rs:44
  watch circular dependency · crates/analysis/src/change_coupling.rs
  watch circular dependency · crates/analysis/src/comparison.rs
  watch packages change together · crates/analysis ↔ crates/cli
  watch one author · crates/analysis
  watch one author · crates/analysis

  next: smackdebt crates/analysis/src/evolution.rs
```

This scope holds thirteen cards, which is past the last rung of the
[one-screen budget](#the-one-screen-budget), so every card states its head and
no evidence at all. `--top 6` below is the same scope with the budget spent the
other way. The two `one author` cards are one package's primary source and its
tests, which are separate contributor-concentration findings.

A path inside a Git repository selects a scope of that repository's report
rather than starting a smaller one: the repository is analyzed and the selection
decides what is answered. Imports resolve against every file the repository
holds, the dependency graph and history are the repository's, and the verdict
frames the selected High count with the analysis-owned sentence `3 of the
repository's 18 high live here.` It omits that sentence when the selection
contains the report's complete selected source inventory, because the
denominator adds no information. The repository root itself prints no such
sentence, and neither does a repository with no High debt. Selecting a path this
way costs a whole-repository run.

A file or directory outside any Git repository has no repository to be a scope
of, so it is inspected on its own and its verdict states no share.

The default terminal view shows at most five affected areas and spends a fixed
one-screen budget on problems, so zooming in changes which problems fill the
screen rather than how much is printed. `--top 10` is the middle level of
detail: in a codebase report it shows up to ten problem cards, in a diff report
up to ten ranked comparisons, and it cannot be combined with `--json` or
`--all`. Use `--all` for all useful debt without those limits, `detail` cards
included. `--all` is not an export: raw dependency edges, references outside the
repository, churn totals, cyclomatic-one values, weak coupling, and healthy rows
never appear in any terminal view. JSON keeps every one of them, so `--all
--json` is rejected.

The default history window is 90 days. It governs every history-derived number,
including activity, churn, change coupling, and contributor concentration:

```console
smackdebt --history 180d
smackdebt --jobs 4
```

## Check a change

Compare the current worktree with the branch it came from:

```console
$ smackdebt diff

smackdebt diff · repository root
  Debt increased in some places and decreased in others.
worse 1 · better 1 · changed 2

AREAS
  crates/output · worse 1 · better 1 · changed 1
  crates/cli · changed 1

FINDINGS
  worse ProblemEvidenceView::serialize · method
        crates/output/src/json.rs:1412
        added · cognitive 12 · cyclomatic 12 · statements 23 · nesting 1 · parameters 2
  better architecture_rows · function
        crates/output/src/output.rs:681
        removed · cognitive 17 · cyclomatic 10 · statements 14 · nesting 3 · parameters 5
  changed assert_index_integrity · function
        crates/cli/tests/unified_acceptance.rs:2436
        statements 118 → 121

  next: smackdebt diff main crates/output/src/json.rs
```

Every diff count is labeled with its word and printed even when it is zero. The
sectioned rows below carry which family moved, so the counts state their words
and their numbers only. Each changed finding shows `path:line` and each changed
measurement as a before and after value. A unit without a source name of its own
is written as its file and its kind, such as `GraphEditor.vue · closure`, so no
generated internal identity reaches a reader.

A diff report keeps `FINDINGS`, `ARCHITECTURE`, and `HISTORY`.
`ARCHITECTURE` names changes in dependency cycles, change reach, and
history-to-code links. Each row names the affected path or pair, shows exact
before and after evidence, and contributes to the diff answer.

`HISTORY` in a diff states only history the change is about. A pair row appears
where the change touched both of its packages, and a contributor-concentration
row where it touched that package; either way the displayed scope has to contain
the subject, so `bow ↔ stern` leaves a view of `bow` and no package-level row
survives a file view. The rows a diff leaves out stay whole in JSON, where the
repository's own coupling and concentration tables are read.

`next:` closes a diff the way it closes a codebase report: it names the command
that reaches the highest-ranked movement's own path, restating the ref the run
compared against. A diff that moved nothing, and a view already at a file, print
no pointer.

The diff tiers are fixed in the same way:

| Tier | Sentence |
| --- | --- |
| `no_debt_change` | No debt changed. |
| `better` | Debt decreased. |
| `worse` | Debt increased. |
| `mixed` | Debt increased in some places and decreased in others. |

The default diff uses one three-row limit across `FINDINGS`, `ARCHITECTURE`,
and `HISTORY`. A mixed result first keeps one row for each direction that is
present, then fills any space left in stable path order. `--top N` is literal:
`--top 1` prints one comparison row. `--all` prints every useful comparison.
No view adds an omitted-row notice. A diff whose verdict is `No debt changed`
while `changed` is not zero still shows one witness row for that count, because
a printed count promises a row a reader can look at.

Adding or deleting healthy code moves no debt. A diff that moves no debt prints
the verdict block and nothing else — here a clean worktree against the commit it
sits on:

```console
$ smackdebt diff HEAD

smackdebt diff · repository root
  No debt changed.
worse 0 · better 0 · changed 0
```

With no ref, Smackdebt tries `origin/HEAD`, `main`, then `master`. It compares
from the merge base through the current worktree, including committed, staged,
unstaged, and non-ignored untracked changes.

Choose a ref or limit the report to a path when needed:

```console
smackdebt diff main
smackdebt diff main crates/analysis
```

The diff report matches named functions and methods by their declared identity.
It matches anonymous callbacks only when a unique assignment, binding, call
site, neighboring label, Ruby example or context description, or unchanged
exact syntax identifies the same unit on both sides. Local anchors include the
nearest declared function or method, so equal local names in separate units do
not collide. Line numbers,
measurements, and source order never decide a match. If anonymous language or
syntax evidence exists on both sides and names several candidates on either
side, Smackdebt keeps one unclear machine row and groups the file into one
warning instead of guessing. A repeated declared name remains an unclear
machine row without that anonymous-unit warning. Any repeated group stays as
additions or removals only when its evidence is absent from the other side.

## Read the report

Severity, direction, and diagnostics are words: `high`, `watch`, `worse`,
`better`, `changed`, `warning`, and `next:`. Every meaning is readable from the
words alone.

In a terminal, each word may be decorated with a one-cell Nerd Font glyph, and
the verdict sentence with a tier-colored `▌` bar. Decoration is resolved from
the same terminal detection as color and has no option of its own: there is no
icon option, no emoji mode, and no theme setting. A glyph always sits beside the
word it decorates and never replaces it.

Piped output is the same report in words: no glyph, no bar, no ANSI, and no
codepoint in the private-use range U+E000–U+F8FF. That makes the output safe to
read with another program without a font or a terminal.

`--color never` produces fully plain output; `--color always` keeps decoration
and styling through a pipe; `NO_COLOR` removes styling while a terminal keeps
its glyphs. Styling reaches decoration only, so removing ANSI sequences yields
the plain bytes exactly.

`COLUMNS` overrides the detected width, and redirected output defaults to 100
columns. Each row chooses its own shape: it stays on one line when its content
fits and stacks its facts on indented lines when it does not. Nothing is
silently clipped at any width — measurements, counts, cycle witnesses, history
evidence, commands, and identities all survive, and a long path continues on the
next line instead of being shortened.

Width changes the shape of a report and never its content. The
[one-screen budget](#the-one-screen-budget) counts slots rather than rendered
lines, so the same invocation states the same problems with the same evidence at
50 columns and at 120. A narrow terminal stacks facts onto extra indented lines
instead of dropping them, which is why a narrow view of the same report can run
longer than one screen.

## Read the problems

A codebase report names problems rather than listing measurements. A problem
card groups findings the report already produced around one anchor — a file, a
cycle, a package, or a package pair — and each finding belongs to at most one
card, so a file with several findings is named once and read once. Clustering
invents nothing: a card's rating is the highest rating among the findings it
claims, every number on it is a number the report already measured, and no card
changes a count or a verdict.

Each row states its rating word, then the pattern name, then the anchor. A file
anchor is its repository-relative path, written `path:line` when the card heads
on a finding with a span; a cycle is anchored on the first step of its witness;
a coupling pair reads `<left> ↔ <right>` and a stable-dependency pair reads
`<source> → <target>`; a package is named by its path, written `repository root`
for the root package. The evidence follows on indented lines: a claimed
finding's `path:line` with its measurements, a size finding's subject and
measured value, `<n> files import this`, `imports <n> files`, `<n> rated units`,
`<n> files in the cycle`, `<n> of <total> files sit in one dependency cycle.`
on the one cycle that is the repository's core, `hot (<n> commits)`, `a change
here reaches <n> files`, `<n> importers follow it`, a coupling pair's commit
operands and dependency state, and a contributor concentration's counts. A
co-change finding names the two files it is about, and its evidence line names
the one the card's head does not. A `hidden_coupling` card heads
`<left> ↔ <right>` and states `changed together in 6 of 9 commits · 67% · no
dependency either way · 4 directories away`, naming neither again. When such a
finding lands on the card of a file that already carried debt, that card's head
names one file, so the line names the other: `changed with
data/store/lib/cache.js in 6 of 9 commits · 67% · no dependency either way · 3
directories away`. A leaking interface's card is the same shape from the other
side, stating `<follower> changed with it in 7 of 12 commits · 58% · 3
directories away` for each importer that follows it.

The pattern ids are the stable vocabulary for an integration; the words beside
them are what a person reads:

| Pattern | The terminal prints | What it needs |
| --- | --- | --- |
| `god_file` | `does too much` | a file that both concentrates rated debt — three High findings, or one High finding among at least six units rated Watch or High — and is broad, meaning it carries a size finding or imports at least ten files |
| `hub` | `everything depends on this`, `depends on many files`, or `change spreads far` | a file whose imports in or out reach eight and, when its package's median is not zero, reach four times that median, or whose reach alone reaches eight; the words state which direction fired. A file that carries debt of its own also needs co-change proof — it is a hotspot in the window, or a change-leakage finding names it — and is otherwise named by that debt instead |
| `tangle` | `circular dependency` | one rated dependency cycle: High across packages, Watch inside one package |
| `hot_mess` | `hot and complex` | a file that carries High debt and is a hotspot in the selected window, five touches by default |
| `shotgun_pair` | `packages change together` | two packages that keep changing together with no code dependency explaining it |
| `bus_risk` | `one author` | a package whose commits concentrate on a single contributor |
| `unstable_dependency` | `depends on less stable code` | a package depending on a less stable package through at least two references |
| `measured` | the finding's own head | anything rated that no other pattern claimed |
| `leaky_interface` | `importers follow its changes` | a file whose importers keep changing with it across at least two directories |
| `hidden_coupling` | `change together without a dependency` | two files that keep changing together across at least two directories with no dependency either way |

`measured` is the fallback, so nothing the report rated disappears for want of a
pattern that recognizes it: its head is exactly the head a finding row used to
print, such as `high compare_units · function ·
crates/analysis/src/comparison.rs:123`. A pattern's own numbers appear as
evidence: an `unstable_dependency` card heads `depends on less stable code ·
crates/api → crates/core` and states `instability 1/4 → 2/3 · 3 imports` below
it, which is the row that used to live in a separate architecture section.

The thresholds behind the named patterns are published the way the rating
thresholds are:

| Rule | Value |
| --- | ---: |
| High findings that make a file concentrated | 3 |
| Units rated Watch or High at which one High finding makes a file concentrated | 6 |
| Files imported that make a file broad without a size finding | 10 |
| File imports in or out at which a file can be a hub | 8 |
| Multiple of its package's median a hub also reaches, when that median is not zero | 4 |
| Directories apart two files must sit before either co-change pattern names them | 2 |
| Commits two files must share before either co-change pattern names them | 5 |
| Share of their commits two files two directories apart must share | 40% |
| Percentage points that bar falls for each further directory between them | 5 |
| Share no distance lowers that bar below | 20% |

A `god_file` needs both halves: concentrated debt alone means a file has bugs,
and breadth alone means a file is large. A `hub` compares a file against the
median of its own package, so the same file is the same problem at every
selected scope. A degree on its own is a shape rather than a problem — a view
imports many components and an error module is imported everywhere because that
is what each is for — so a file that carries debt keeps its `hub` card only
where co-change says the degree costs something, and is otherwise named by the
debt it holds. A file with no debt of its own keeps a `detail` `hub` card
whatever its history, so a degree stays inspectable under `--all` and in JSON.
The two co-change patterns share their three thresholds and are explained in
[Read how far a change reaches](#read-how-far-a-change-reaches).

Every card is either a `default` card or a `detail` card. A card is `default`
when it claims at least one finding that affects the verdict, when it claims a
change-leakage finding, or when it anchors a rated architecture, coupling,
contributor-concentration, or stable-dependency finding. Every other card is
`detail`: it appears under `--all`, at its own anchor when that anchor is the
selected scope, and always in JSON. A widely imported file with no debt of its
own and nothing following its changes, a file whose whole debt is advisory or
non-primary, and a file whose only evidence is its size are all `detail` cards.
That same widely imported file becomes a `default` card as soon as a leakage
finding names it, because a file whose importers keep changing with it is a
problem even when the file itself measures clean.
Visibility decides where a card is shown and never what it is — it moves no
rating, no count, and no verdict, and hiding a `detail` card never reorders the
cards that remain.

A file's leakage numbers appear as extra lines on the card that already names
that file, and a card of their own exists only when nothing else names the file
or the pair, so the report never states one subject twice.

Cards are ranked against each other, so the first card is the problem to look at
first. The order is the card's rating, then primary application source before
cards with no source finding and non-primary source, then how many High findings
it claims, then hot before not hot, then how many findings it claims, then the
pattern, then the accepted finding rank of its top claimed finding, then the
anchor's path and line. Architecture and history cards have no source finding,
so they keep the middle position deliberately. Rating stays strongest: a High
benchmark problem still appears before a Watch primary problem, and no role is
hidden.

### The one-screen budget

Codebase debt detail fits about one screen at every scope, so following the
report's own `next:` line never produces a longer report than the one that
suggested it. The budget is 24 slots, where a card costs one slot plus one slot
per evidence line it shows. It is spent through a ladder of card-count and
evidence-line pairs — `(6, 3)`, `(8, 2)`, `(12, 1)`, `(24, 0)` — applying the
first rung whose card count is at least the number of cards the displayed scope
holds. Every rung costs exactly the budget, so showing more problems means
showing less evidence for each. A scope holding more than twenty-four cards
shows the twenty-four highest ranked and cuts the rest without a bookkeeping
row; `--top` is how you lift that cut.

`--top N` selects the rung `N` selects, by the same rule the default applies to
the scope's own card count, so a larger `N` buys breadth by spending evidence
depth and a smaller one does the reverse. The view of this scope above holds
thirteen cards and therefore allows no evidence lines at all; the same scope at
`--top 6` allows three, and a card with less evidence than that simply states
what it has:

```console
$ smackdebt --top 6 crates/analysis

smackdebt · crates/analysis
  Worn in the usual places.
  3 of the repository's 18 high live here.
  A typical change here touches 4 files.
3 high · 17 watch · 1,585 checked
worst: crates/analysis/src/evolution.rs — hot AND complex

PROBLEMS
  high change spreads far · crates/analysis/src/evolution.rs
        crates/analysis/src/evolution.rs:60 · method · parameters 13
        a change here reaches 14 files
        file · 1,848 lines
  high everything depends on this · crates/analysis/src/report.rs
        crates/analysis/src/report.rs:1904 · function · cognitive 23 · cyclomatic 15
        11 files import this
        imports 13 files
  high ArchitectureGraph::new · method · crates/analysis/src/architecture.rs:17
        parameters 6
        file · 1,172 lines
        hot (10 commits)
  high cycle_witness · function · crates/analysis/src/cycle_witness.rs:3
        cognitive 34 · cyclomatic 15 · nesting 5
  high strongly_connected_components · function · crates/analysis/src/
        strongly_connected_components.rs:1
        cognitive 30 · cyclomatic 15 · nesting 4
  watch change spreads far · crates/analysis/src/comparison.rs
        crates/analysis/src/comparison.rs:52 · method · parameters 7
        a change here reaches 11 files
        hot (5 commits)

  next: smackdebt crates/analysis/src/evolution.rs
```

The head states how far a typical change to this scope travels, because that is
a fact about the scope the head already names. How far a change *reaches* is a
claim about the dependency graph, so it stays on the row or card that says which
code to look at — here, two `change spreads far` cards.

A cycle witness is the one exception to the accounting: it costs one slot
however many steps it stacks, because eliding a witness destroys the fact rather
than shortening it. A selected file is the other: it shows every card anchored
on that file with complete evidence and no ladder at all, because drilling to a
file is itself a request for detail.

```console
$ smackdebt crates/analysis/src/change_coupling.rs

smackdebt · crates/analysis/src/change_coupling.rs
  Clean. Ship it.
0 high · 0 watch · 22 checked

PROBLEMS
  watch circular dependency · crates/analysis/src/change_coupling.rs
        crates/analysis/src/change_coupling.rs
        → crates/analysis/src/evolution.rs
        → crates/analysis/src/change_coupling.rs
        2 files in the cycle
        a change here reaches 14 files
        hot (13 commits)
  watch packages change together · crates/analysis ↔ crates/cli
        changed together in 33 of 98 commits · 34% · no direct dependency · linked via crates/output
  watch one author · crates/analysis
        one contributor made 57 of 60 commits
```

The verdict counts rate units: `0 high · 0 watch · 22 checked` counts the
functions, methods, and closures in scope. A cycle, a coupling pair, and a
contributor concentration are none of those, so a scope whose units are all
healthy can still hold problems worth reading.

If you miss a per-finding row you used to see, one card now claims that file's
findings and states them as its evidence. JSON keeps every claimed finding as
its own row, and each card names exactly which rows it claimed.

## Read the ratings

Smackdebt reports separate signals instead of hiding them in one score.

| Signal | Watch | High |
| --- | ---: | ---: |
| Cognitive complexity | 15 | 25 |
| Cyclomatic complexity | 11 | 21 |
| Statements in a function | 50 | 100 |
| Maximum nesting depth | 4 | 7 |
| Declared parameters | 6 | 9 |

A function takes its highest signal rating. Terminal views focus on functions
that need attention. JSON retains healthy, watch, and high counts for every
scope. Neither view averages one bad function into a reassuring project score.

Cognitive complexity starts at zero. A control-flow break adds one plus its
nesting depth. Alternatives and labeled jumps add one. A run of `&&` or `and`
adds one, and changing that run to `||` or `or` adds another. A separately
rated closure or nested function does not increase its parent's value.
Recursion is not inferred from syntax alone.

Maximum nesting depth is the deepest level reached inside one rated unit,
counted from the same nesting events cognitive complexity uses. Parameter count
is the number of parameters a unit declares. A separately rated closure reports
its own depth and does not deepen its parent.

Cyclomatic complexity starts at one and adds one for each independent decision.
Because it starts at one, a cyclomatic value of one is never printed.
Statements are counted by syntax, not physical lines: `a(); b();` counts as
two, while one statement spread over several lines counts as one. Blank lines,
comments, wrappers, markup, and nested rated units do not count.

These equivalent functions each have cognitive complexity 1, cyclomatic
complexity 2, and two logical statements:

```rust
fn choose(ready: bool) -> i32 {
    if ready { return 1; }
    0
}
```

```ruby
def choose(ready)
  return 1 if ready
  return 0
end
```

Recent activity comes from distinct non-merge commits that touched a file in
the selected history window. Smackdebt uses activity to order code that already
needs attention; activity does not change the code-health rating.

## Read the architecture

The same command also reports static dependency health. Smackdebt extracts
imports, includes, modules, and requires during the source parse, then resolves
them against discovered repository files:

- `internal` references identify exactly one file in the repository;
- `external` references name code outside the repository;
- `unresolved` references are dynamic or malformed;
- `ambiguous` references match several possible internal files;
- `asset` references name a file that is not source, such as a YAML fixture or
  an image imported for its bytes.

Smackdebt does not guess when identity is unclear. Unmatched and ambiguous
references are counted in one grouped warning sentence, their per-reference
detail appears under `--all` or when the selected scope is a file, and JSON
retains their locations and reasons. At every other scope that grouped sentence
is their whole terminal presence.

An asset reference is not one of them. Smackdebt inventories source files only,
so a relative import whose name carries an extension no source language claims —
`./config.yaml?raw`, `./logo.svg`, `./theme.css` — could never have matched a
discovered file, and reporting it as an import that could not be followed would
claim a hole in the dependency graph the code does not have. Such a reference
never enters that count, never appears in the terminal, and never marks its
package incomplete; JSON keeps its row with `"kind": "asset"` and the reason
`target is an asset`. The rule reads the paths the reference was looked for
under, not the name as written, so dotted module notation — Python's
`from ..core import thing`, Java's `import app.Local` — is never mistaken for a
file with the extension `core` or `Local`; a missing module stays a hole. A
reference with no extension, or with a source extension that matches no file,
stays unresolved exactly as before. Nothing on disk is consulted, so a name
whose own stem carries a dot settles as an asset: an import written
`./webpack.config`, for an absent `webpack.config.js`, is filed as an asset
because `config` is an extension no language claims. Its row and its location
are kept; only the count of holes declines to guess.

`dependency_coverage` has no asset counter. Asset references are counted under
`context_relations`, with the other relations that are real evidence but never
enter the verdict graph, so the partition still totals every reference the parse
found. To count assets alone, count the `resolution_diagnostics` rows whose
`kind` is `asset`.

A reference written against the name a package declares for itself resolves
inside the repository. When no repository path matches a reference, Smackdebt
compares its first segment with the names packages declare in their manifests:
`Cargo.toml` `[package] name` with a `[lib] name` override, `package.json`
`name` including a scoped `@scope/name`, `pyproject.toml` `[project] name`, and
a gemspec name. Rust treats hyphens and underscores as the same character; other
ecosystems compare declared names exactly. The reference becomes an internal
`uses` edge only when exactly one package in the repository declares that name.
When two packages declare the same name, the reference stays unresolved with its
diagnostic instead of guessing. Smackdebt reads declared names only: it never
executes build configuration and never emulates lockfiles, resolver algorithms,
workspace inheritance, or version constraints.

A cycle crossing packages is High. A file cycle contained in one package is
Watch. Either becomes one `circular dependency` card that states how many files
the cycle holds and carries its closed witness one step per line, so the
evidence is never shortened away. Fan-in is the number of packages that depend
on a package; fan-out is the number it depends on. Instability is `fan-out /
(fan-in + fan-out)` and is printed as its exact integer fraction, so a package
that depends on less stable code reads `instability 1/4 → 2/3`. A `hub` card
counts files rather than packages: `<n> files import this` and `imports <n>
files` state the anchor file's own degree.

Architecture verdicts describe the code that ships. Package dependency edges,
package and file cycles, fan-in, fan-out, instability, and stable-dependency
findings are built from primary-role relations only. Test, example, and
benchmark relations stay complete in the machine report as context: they appear
in JSON and in `--all`, they still count toward dependency coverage, and they
still explain change coupling, so a package pair linked only by a test import is
never reported as coupling with `no code dependency`.

A Rust reference declared under a `#[cfg(test)]` scope carries the test role even
when the file around it is production source. The rule is syntactic: Smackdebt
matches a `cfg` attribute on the reference's own item or on any enclosing `mod`
whose predicate names `test` outside a `not(...)`, so `#[cfg(test)]`,
`#[cfg(all(test, not(loom)))]`, and `#[cfg(any(test, fuzzing))]` match while
`#[cfg(not(test))]`, `#[cfg(feature = "test")]`, and `#[cfg_attr(test, ...)]` do
not. Smackdebt does not evaluate configuration predicates and never learns which
features a build enables. A file whose module declarations are all test-scoped —
the file named by `#[cfg(test)] mod tests;` and nothing else — is test source
itself.

Rust module wiring is not a cycle. A Rust file that is not `mod.rs`, `lib.rs`, or
`main.rs` owns a directory of its own name, so `mod child;` in `a.rs` names
`a/child.rs` rather than a sibling. When two files already own each other through
a module declaration, the imports between that pair are excluded from the file
cycle graph. The exclusion is limited to that pair: every other relation stays,
so a cycle that merely passes through an owning pair by way of other files still
reports, and cycles between sibling modules are untouched.

Code and architecture results remain separate. Lower source complexity does not
cancel an introduced package cycle. In diff output an introduced cycle is
worse, a removed cycle is better, and an ordinary edge change is changed.

Dependency edges appear in no human view, at any scope and any detail level,
`--all` included. A relationship reaches the terminal only as a problem card's
aggregate evidence — a fan-in or fan-out count, a cycle's member count, or a
cycle witness — so the JSON relation tables are the only place to read the edges
themselves. The per-import and module-ownership rows are gone with the section
that carried them: over a 296-file directory they produced about 1,270 unrated,
unranked rows and named no problem, which is why zooming in used to make the
report longer.

Pass a package, directory, or file path to read the problems anchored there
instead:

```console
smackdebt crates/analysis
smackdebt --all crates/analysis
```

Unresolved and ambiguous references keep their own rows — `could not be matched`
and `matched more than one file` — under `--all` or at a file scope. Non-primary
roles and advisory evidence appear only when they change how a row should be
read.

Static analysis does not provide compiler type resolution, runtime tracing, or
executed build configuration. Macros, generated paths, runtime imports, and
unsupported aliases can therefore remain unresolved.

## Read how far a change reaches

Architecture evidence stays next to a named subject: an area row, a problem
card, or an architecture diff row. The verdict does not state a repository-wide
graph number without saying where to act.

| Sentence | Where it appears | What the numbers count |
| --- | --- | --- |
| `a change here can reach 5 of 12 packages` | the named package area | packages that transitively depend on this package, counting this package |
| `a change here reaches 17 files` | the named file or cycle problem | files anywhere in the repository that transitively depend on the named source, excluding that source |
| `9 of 86 files sit in one dependency cycle.` | the card of the cycle that is the core | members of the largest cycle out of the files the dependency graph is built over |

Reach counts dependants across the whole repository, not only the named source's
own package: a file that a second package imports is reached from the first one.
Package reach is the exception in the other direction — it is a count of
packages, and the area row states it for the widest package alone.

The core is a superlative over the whole graph, so it belongs to exactly one
card: the tangle whose own members are that cycle. Every other cycle states its
own size instead, as `9 files in the cycle`. Two things therefore leave a
material core stated nowhere in the terminal, and it stays a JSON fact in both:

- **No card exists.** A cycle finding is raised per package, so a core whose
  members span more than one package has no card to land on at any scope.
- **The card exists but is not shown here.** Problem cards compete for the
  [one-screen budget](#the-one-screen-budget), and the rung a scope lands on may
  allow the card no evidence lines — or cut the card itself. A large
  repository's root commonly hides its own core this way while a smaller scope,
  or `--all`, states it. `--top` changes which rung applies rather than
  guaranteeing the card: raising it past twenty-four buys breadth at the cost of
  every evidence line.

Neither case is counted as withheld. `graph_evidence.suppressed_core` counts one
core only when an incomplete dependency graph made the superlative unsafe to
state at all; a core the budget did not reach was decided, and JSON carries it
in `core_component` whatever the terminal had room for.

How far a *typical* change travels is measured from the scope's own commits
rather than from the graph, so it needs no further subject and rides under the
tier sentence at every scope that has one:

| Sentence | Where it appears | What the numbers count |
| --- | --- | --- |
| `A typical change here touches 4 files.` | the verdict head, at a repository, package, or directory scope | the nearest-rank median of the files each commit touching this directory changed |

These facts do not change a codebase tier or rating. Each is absent when its
evidence is incomplete, the repository is too small, or the number is too weak
to mean anything: package reach needs at least 3 packages and a reach of at
least 2, a package's file reach needs at least 20 files, a core needs at least 5
files and 2% of the graph, and a typical change needs at least 10 commits and a
median of at least 3 files. Every graph fact also needs a complete dependency
graph; a typical change needs a complete history stream instead. A file scope
states no typical change, because a per-file histogram would state sample noise
as a fact, and a diff states none at all, because a diff answers about a change
rather than about a tree.

Every graph count here counts files of one kind: primary, parsed files — the
files the dependency graph is built over, which is what every other dependency
number in the report counts too. Tests, examples, benchmarks, fixtures, and
generated files are outside both halves of a fraction, so a package that holds
41 files may read `36`, and a package with a large test suite may read about
half its file count. A fraction whose halves came from two populations would
answer nothing, which is why the denominator is the graph rather than the
directory listing. Typical change size is the exception: it is counted from
commits rather than from the graph, so it counts whatever files a commit
touched.

Two problem patterns come from the same family, joining what changed together
with what depends on what:

- `importers follow its changes` names a file whose importers keep changing with
  it. The evidence reads `3 importers follow it` and then one line per importer:
  `web/src/follower.js changed with it in 7 of 12 commits · 58% · 3 directories
  away`. An interface whose callers must be edited whenever it moves is leaking
  its internals into them.
- `change together without a dependency` names two files that keep changing
  together where nothing connects them: `changed together in 6 of 14 commits ·
  43% · no dependency either way · 8 directories away`. This is the front end
  and the API it calls over HTTP, or two implementations of one format — a
  contract the code does not express.

Both require the two files to sit in different directories, and a larger
distance lowers the agreement each needs, because distance is what makes
co-change surprising: two directories apart requires 40% of their commits
together, three requires 35%, and six or more requires 20%. Both need at least
five shared commits, so an anecdote is not a pattern.

A pair is reported as hidden only when the absence of a dependency was proved.
Smackdebt walks the connection graph — every import and every module
declaration, in both directions — from one end of the pair and, if that walk
runs out of budget, from the other. An inconclusive search reports nothing,
because absence is proved rather than assumed.

A wiring file — `lib.rs`, `mod.rs`, `index.js`, `index.cjs`, `index.mjs`,
`index.ts`, and `__init__.py` — is never named as a leaking interface. Such a
file is a list of declarations and re-exports rather than behavior, so it holds
no abstraction to leak, and its importers change with it because adding an
export and using it is one edit. Nothing else is excluded: a program entry point
such as `main.rs`, and an `index.vue`, `index.jsx`, or `index.tsx` — a
directory's own component or an application bootstrap rather than a barrel — all
hold behavior, so importers following them is still worth reporting.

These signals come from history, so they never enter the ratchet gate, which
only counts signals that do not move with wall-clock time. A diff can report
trusted movement in reach, core size, and change leakage because it compares
the current and base graphs. Each reach row compares one named package or file
with itself, and each core row compares the dependency components containing
one file present on both sides. Change amplification remains codebase-only.

## Read the history evidence

History evidence comes from locally available non-merge Git history inside the
selected `--history` window. The window governs every history-derived number:
activity, churn, change coupling, and contributor concentration all describe the
same commits. History keeps its evidence separate from current code health and
the static dependency graph:

- activity counts distinct commits that changed a current file or package;
- churn reports textual lines added and deleted;
- change coupling reports packages that changed in the same commits;
- contributor count reports how many normalized contributors changed a
  package;
- top-contributor share reports the largest contributor's package commits over
  all contributor commits for that package.

Coupling uses Jaccard similarity: shared commits divided by commits touching
either package. Default findings require at least three shared commits and 20%
similarity. Weaker observations stay in JSON only. Recurrent coupling without a
static dependency in either direction is Watch because it can reveal a missing
or unclear package relationship.

In a codebase report, actionable history is problem cards: one `packages change
together` card per unexplained coupling pair, one `one author` card per
contributor concentration, and the two file-level co-change patterns described
in [Read how far a change reaches](#read-how-far-a-change-reaches), each keeping
its finding's exact evidence and each
ranked against every other problem rather than sitting in a section of its own.
A `one author` card states counts only, such as `one contributor made 57 of 60
commits`. A coupling pair that a code dependency already explains is context
rather than debt: it produces no finding, so no card names it at any scope or
detail level, `--all` included, and its complete row stays in the machine
report. Churn totals stay in JSON; the terminal reports activity where it
changes a decision, as `hot (n commits)` evidence on a card.

A diff report keeps its `HISTORY` section this release, with at most three
actionable rows ordered by shared commits, then similarity, then stable package
identity, and with the contextual pairs it shows today.

A package pair is reported once. Source role and trust variants are aggregated
into that one card, and per-role history stays in JSON. A scope and its own
ancestor never form a pair, because commits they share are structural rather
than hidden coupling. Every coupling evidence line states the shared and total
commit counts, the similarity, and how the package dependency graph links the
pair. `code dependency exists` means a trusted eligible `uses` relation
links the pair in either direction, including relations resolved through a
declared manifest name and relations whose role is test, example, or
benchmark. `no direct dependency` with `linked via <package>` means no such
relation exists but a dependency path connects the pair in one direction,
naming the first intermediate package on a shortest such path. `no code
dependency` means neither a direct relation nor a dependency path exists in
either direction. The link informs the wording only: a pair without a direct
relation keeps its Watch finding whether or not an indirect path exists.
Coupling explanation is a claim about the repository rather than about
production code, so it keeps the wider set of relations that architecture
verdicts leave out.

Smackdebt follows detected renames back from files that still exist and assigns
their history to the files' current packages. It does not reconstruct removed
files or old package layouts. A file renamed twice is followed only while the
chain of detected renames is unbroken, so history before a gap is not counted:
co-change under-reports for such a file rather than guessing, and the gaps are
counted in the machine report. Binary changes count as commits without invented
line totals. Shallow, partial, empty, or unavailable history is stated in the
report while source and static architecture results remain usable.

How far apart two files sit is read from their repository-relative paths by
splitting on `/`, and a pair inside one directory is never recorded. On a
Windows checkout those paths are written with the platform separator, so every
file appears to sit in the repository root: the file co-change table stays
empty, the two patterns built on it report nothing there, and the
typical-change sentence survives only at the repository root, because no
package or directory scope has a directory of its own left to read. Package
coupling, churn, activity, and contributor concentration are unaffected.

Contributor names, addresses, and internal identities stop before the report.

## Checked command examples

These short examples run against generated public repositories in the release
evidence. Each comment declares the exact exit status, empty stderr, and the
stable stdout fragments that must appear in the stated order.

<!-- smackdebt-example fixture=evolution status=0 stderr=empty stdout=smackdebt_·_repository_root|checked|PROBLEMS|packages_change_together_·_a_↔_b -->
```console
smackdebt --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=smackdebt_diff_·_repository_root|Debt_increased_in_some_places_and_decreased_in_others.|worse|better|changed|AREAS|FINDINGS -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased_in_some_places_and_decreased_in_others.|worse_b|better_a|changed_c|next:_smackdebt_diff_main_b/main.js -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased.|worse_package_dependency_cycle_introduced -->
```console
smackdebt diff main c --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_decreased.|better_a|next:_smackdebt_diff_main_a/main.js -->
```console
smackdebt diff main a --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=No_debt_changed. -->
```console
smackdebt diff main new/untracked.js --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=comparison-trust-warning status=0 stderr=empty stdout=No_debt_changed.|Not_all_source_was_checked.|0_of_1_source_files_were_analyzed.|WARNINGS|1_source_file_uses_an_unsupported_language. -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=comparison-trust-warning status=0 stderr=empty stdout=smackdebt_·_page.astro|Not_all_source_was_checked.|0_of_1_source_files_were_analyzed.|1_source_file_uses_an_unsupported_language. -->
```console
smackdebt page.astro --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=comparison-trust-warning status=1 stderr=smackdebt:_path_not_found:_does/not/exist stdout=empty -->
```console
smackdebt does/not/exist --color never
```

<!-- smackdebt-example fixture=comparison-trust-warning status=1 stderr=smackdebt:_no_source_files_found_under:_docs stdout=empty -->
```console
smackdebt docs --color never
```

<!-- smackdebt-example fixture=comparison-trust-warning status=1 stderr=smackdebt:_not_a_source_file:_README.txt stdout=empty -->
```console
smackdebt README.txt --color never
```

<!-- smackdebt-example fixture=generated-javascript status=0 stderr=empty stdout=No_debt_changed.|1_file_has_anonymous_units_that_could_not_be_matched_safely. -->
```console
smackdebt diff HEAD~1 bundles/collision.bundle.js --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=generated-javascript status=0 stderr=empty stdout=FINDINGS|transitions/from-generated.js -->
```console
smackdebt diff HEAD~1 transitions/from-generated.js --all --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased_in_some_places_and_decreased_in_others.|FINDINGS|worse_b|next:_smackdebt_diff_main_b/main.js -->
```console
smackdebt diff main --top 1 --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased_in_some_places_and_decreased_in_others.|FINDINGS|ARCHITECTURE|HISTORY|next:_smackdebt_diff_main_b/main.js -->
```console
smackdebt diff main --all --color never --jobs 1 --history 36500d
```
Terminal and JSON output contain only aggregate contributor counts and
concentration operands. History and all other analysis stay on the local
machine.

Diff reports show history as existing context for changed files and packages.
Historical values are not labelled better or worse. When a worktree dependency
changes the finding, the terminal says the packages `now change together
without a code dependency` or `no longer change together without a code
dependency`. The retained history values do not change.

## Discovery

Smackdebt finds the repository root, supported source files, ignored paths, and
project packages. It recognizes packages from common manifests, including:

- `Cargo.toml`
- `package.json`
- `pyproject.toml`
- `pom.xml`
- Gradle settings and build files
- `CMakeLists.txt`
- `Gemfile` and `*.gemspec`

Several recognized manifests in one directory describe one package with
several ecosystem markers. Files belong to their nearest package. Repositories
without a known manifest get one root package. Git ignore rules apply by
default, along with explicit exclusions and dependency directories. A supported
file is not ignored only because its directory looks generated.

An explicit path stays the selected path. A supported or recognized
unsupported source file produces a file report, and a directory containing
source produces a report for that directory. Smackdebt does not replace an
explicit path with repository results when the path has nothing to analyze.
Inside a repository, that inspection reads source only from the selected file
or directory subtree while keeping displayed paths repository-relative.

Every selected file has one source role: primary, test, example, benchmark,
fixture, generated, vendored, or dormant. Classification checks explicit
`source_roles` configuration first, then language-owned generated markers. It
next treats
`.min`, `.bundle`, and `-bundle` names as generated for `.js`, `.mjs`, and
`.cjs` files. JavaScript, JSX, TypeScript, and TSX source is also generated when
it is at least 65,536 bytes and averages at least 512 bytes per nonempty
physical line. That content check uses the source already read for analysis;
Vue documents do not use it. A `.js`, `.mjs`, or `.cjs` file whose name begins
with `jquery` is vendored, which is the one library family named outright.
Generic filenames and paths follow, then a Rust
file whose every module declaration is test-scoped, and finally primary.
Different matches at the same level are an
invalid configuration and exit with status 2. Primary, test, example, and
benchmark source affect default verdicts. Fixture, generated, vendored, and
dormant source remain
visible in JSON, `--all`, and explicit file inspection without affecting
verdicts, default problems, root worst-offender selection, or navigation.
Explicit configuration wins over generated evidence. Directory names alone do
not assign the generated, vendored, or dormant role, so authored source under
`public`, `share`, or `assets` remains authored unless another rule matches it.

<!-- smackdebt-example fixture=generated-javascript status=0 stderr=empty stdout=smackdebt_·_bundles/vendor.min.js|0_high|PROBLEMS|generated -->
```console
smackdebt bundles/vendor.min.js --color never
```

Two rules move JavaScript out of the verdict, and they use two different words
because they know two different things.

**Vendored** is a claim about who wrote a file, so only a name makes it: the
`jquery` family above. Nothing else is guessed at, because absence of use is no
evidence of authorship.

**Dormant** is a claim about attention, and it is made only from what was
measured. A `.js`, `.mjs`, or `.cjs` file is dormant when all of these absences
hold at once: the file declares no `import` and no `export` of its own, nothing
in the repository imports it, no package manifest names it as something it
publishes, installs, or runs, its name is not a conventional entry name such as
`index.js` or a tool configuration name such as `*.config.js` or a dotfile, and
no commit inside the history window touched it. The rule is skipped entirely
when the history window holds no commits, because a window with nothing in it
proves nothing about any file. Dormancy is measured rather than declared, so
`[source_roles]` has no `dormant` key; to overrule it, name the file `primary`.

The module test decides which files the dormancy rule may look at. A file that
states its own imports and exports is one the dependency graph can speak about:
nothing importing it makes it an orphan, which the report already says. A file
that states neither is a script a page or a build tool loads by name, so no
import could ever have named it, and having no importer is what it is supposed
to look like. TypeScript and JSX spellings are never considered, however cold or
unimported they are, because both compile from source the repository authored.

What dormancy does not say is who wrote the file. A page script the repository
wrote years ago and has not opened since answers to every signal above, and is
called dormant for exactly that reason — not because anyone decided it was
third-party. Its findings stay in JSON, in `--all`, and in its own file report,
so nothing is lost; only the default verdict stops counting it. Widening
`--history` or naming the file under `[source_roles] primary` restores it.

<!-- smackdebt-example fixture=source-roles status=0 stderr=empty stdout=smackdebt_·_share/jquery.plugin.js|0_high|PROBLEMS|vendored -->
```console
smackdebt share/jquery.plugin.js --color never
```

Files that cannot be parsed stay visible in the coverage summary. Smackdebt
does not quietly count them as healthy.

## Languages

One tree-sitter source engine supports:

- C and C++
- Java
- JavaScript and JSX
- Python
- Rust
- TypeScript and TSX
- Ruby
- Vue single-file components, including script and template regions

Astro and Kotlin files remain visible as unsupported coverage; they are not
counted as healthy. Astro documents stay in codebase and diff inventories, but
Smackdebt does not parse Astro yet. Language dispatch is compiled into the binary. Each language translates
its own syntax into the same cognitive, cyclomatic, and logical-statement rules.
Exact fixtures check units, recovery, spans, nesting, and measurements before a
language is listed here.

## JSON

Use `--json` for CI scripts, editors, and other tools:

```console
smackdebt --json
smackdebt diff main --json
```

The object answers the common question first, so `smackdebt --json | head`
is useful on its own and no consumer joins a table to learn whether the code is
in trouble:

```json
{
  "schema_version": 4,
  "mode": "codebase",
  "verdict": { "tier": "worn", "sentence": "Worn in the usual places.", "mode": "codebase" },
  "summary": {
    "checked": 2166, "high": 17, "watch": 58, "high_architecture": 0,
    "debt_diff": { "worse": 0, "better": 0, "changed": 0, "total": 0 },
    "worst": [
      { "path": "crates/project/src/project.rs", "name": "analyze_diff",
        "container": null, "unit_kind": "function", "reason": "hot_and_complex" }
    ]
  }
}
```

`verdict.tier` is the frozen id and `verdict.sentence` is the exact sentence the
terminal prints for it. In a diff the tier and sentence are the diff answer and
`summary.debt_diff` states each count with its word, including zero counts.
`summary.worst` names at most three offenders with real repository-relative
path strings. Every value in the head also exists in a table, and both come from
the same completed report.

Each source comparison has `participation: "verdict"` or
`participation: "context"`. Context comparisons remain available for tools and
file inspection but do not enter `summary.debt_diff`. The value is derived from
both sides of the comparison, so a change between generated and primary roles
is explained without treating generated source as debt movement.

When any selected source file was not analyzed, the verdict block always says:

```text
Not all source was checked.
<analyzed> of <selected> source files were analyzed.
```

One missed file is enough; there is no percentage threshold. JSON puts the same
`sentence` and `detail` in `verdict.qualifier`, together with integer
`selected_files` and `analyzed_files` counts. Coverage tables retain the same
counts plus unsupported and failed file detail.

A retained sub-scope from a completed root report may carry `verdict.share`,
holding the same sentence the terminal prints and both integer counts behind it.
A retained sub-scope with the same selected-file total as the root omits the
member because its denominator adds no information.
A report of a file or directory outside any Git repository omits this member
because its limited inventory did not measure repository High debt. The member
is also absent at the repository root and when the repository holds no High
debt:

```json
"verdict": {
  "tier": "clean", "sentence": "Clean. Ship it.",
  "share": { "sentence": "0 of the repository's 20 high live here.", "high": 0, "repository_high": 20 },
  "mode": "codebase"
}
```

`verdict.reach`, `verdict.core_size`, and `verdict.amplification` keep the
aggregate values for machine consumers: `reached` and `total`, `core` and
`files`, `median` and `commits`. Only `amplification` is a terminal verdict
line; the two graph facts reach a reader on the area row and the cycle card
that name a subject. Each member is absent when its evidence is too weak, so a
consumer reads presence rather than a zero:

```json
"verdict": {
  "tier": "worn", "sentence": "Worn in the usual places.",
  "reach": { "sentence": "A change in one package can reach 6 of 12 packages.", "reached": 6, "total": 12 },
  "core_size": { "sentence": "9 of 86 files sit in one dependency cycle.", "core": 9, "files": 86 },
  "amplification": { "sentence": "A typical change here touches 4 files.", "median": 4, "commits": 77 },
  "mode": "codebase"
}
```

`verdict.reach` answers about whatever the selection is, so the sentence it
carries differs by scope: the repository root closes over packages and reads
`A change in one package can reach 6 of 12 packages.` — the same numbers the
widest area row states — while a package closes over its own files and reads
`A change here can reach 17 of 36 files in this package.`, which is an aggregate
no terminal row states, because a package-wide number names no file to look at.

`verdict.core_size` is a repository-root member only: a sub-scope holds part of
a graph and so states no superlative over the whole of it. The terminal is the
other way round — the core rides the card of its cycle, and that card appears at
every scope its member files belong to. So a report of one file inside the core
carries the sentence in `problems` and a null `verdict.core_size`. A consumer
that wants the core regardless of selection reads `core_component`, which names
every member whatever scope was asked for.

The `problems` table holds every card in problem-rank order, `detail` cards
included, and a row's position is that card's identity. `pattern` is one of the
frozen ids, `visibility` is the string `default` or `detail` rather than a
boolean, `anchor` names its kind and carries the file, file list, package, or
package pair that kind implies, `evidence` preserves the order the terminal
prints, and `claimed` names every finding the card took, table by table:

```json
{
  "pattern": "tangle", "rating": "watch", "visibility": "default",
  "anchor": { "kind": "files", "files": [2, 8] },
  "evidence": [
    { "kind": "members", "value": 2 },
    { "kind": "architecture_findings", "index": 0 },
    { "kind": "hot", "value": 9 }
  ],
  "claimed": [{ "table": "architecture_findings", "index": 0 }]
}
```

Every index resolves inside the table it names, no finding is claimed by two
cards, and no retained finding of a claimable table is left unclaimed — so the
cards are a complete partition of the debt the report retained, and a consumer
can join from a card to its rows or from a row back to its card.

Behind the head, JSON contains the complete report, including everything the
terminal leaves out: raw dependency edges, references outside the repository,
churn totals, weak coupling, weak file change coupling, hotspots, size findings,
orphan files, file change coupling, package closures, candidate file reach, and
healthy counts. Dependency edges appear in no human view at any scope or detail level,
so the relation tables are the only place to read them; the terminal states a
relationship as aggregate card evidence such as a fan-in count or a cycle
witness. One package table owns stable package IDs, repository-relative paths,
current or base-only presence, and the name a manifest declares; machine path
`.` stays unchanged even though terminal output calls it `repository root`.
Files expose SourceRole, parse outcome, and trust. Recovered Watch and High
facts remain advisory in JSON and `--all` without entering health, default
findings, architecture verdicts, coupling, or diff verdicts.

Static relations identify `uses` or `module_ownership` independently from
role, trust, resolution, and source spans. One file pair can carry more than one
relation row, because a file that imports a module normally and again inside a
`#[cfg(test)]` module produces one primary and one test relation. History rows expose role-aware churn,
coupling operands, and contributor concentration without contributor identity.
Hotspots, size findings, orphan files, stable-dependency findings, and
knowledge-concentration findings each own a table and state their own `kind`.

The `graph_evidence` object says whether architecture claims are safe to show.
It records affected packages, parse and resolution failures, invalid resolution
configuration, and the number of reach, core, or leakage facts withheld from
human output. Weak graphs stay inspectable in JSON without creating confident
terminal claims.

Four tables carry the change-reach facts. `file_change_coupling` holds every
retained file pair with the lower file index first, its shared and union commit
counts, and the directory distance between the two files.
`change_leakage_findings` holds the pairs a rule named, each with its `kind`,
the `coupling` row it was decided from, and, for `leaky_interface`, the
`interface` file. `package_closures` holds one row per package whose file reach
is material, with the `source` file that has the largest reach, the `files` the
package's dependency graph is built over, and that exact `reach`; a package
below the file floor or above the closure limit has no row, so a consumer joins
by package rather than by position. `file_reach` holds exact reach for the
selected candidate files. `core_component` names every member of a material
dependency core.

Diff JSON adds `propagation_comparisons`, `core_comparisons`, and
`change_leakage_comparisons`. Each row carries a direction, a named file or
pair, and exact before and after evidence where it applies. `comparison_ref`
keeps the ref used to build the comparison. Scope rows contain the matching
comparison IDs for direct navigation.

Diff JSON also carries `diff_graph_evidence`, with separate `current` and
`base` graph status. Its propagation, core, and leakage suppression rows count
candidate comparisons withheld before filtering, including how many depended
on incomplete current evidence, base evidence, or both. Withheld movement never
enters a comparison table or the diff verdict.

A retained pair that produced no finding reaches no human view at all, `--all`
included: the weaker pairs are JSON-only by design, and `file_change_coupling`
is where to inspect them.

If a row the terminal used to print has disappeared, the version-4 table that
retains it is where to look: `dependency_edges` and `package_edges` for import
rows, `change_coupling` for the pairs a code dependency explains, `activity` and
`file_history` for churn, `external_dependencies` for references outside the
repository, and `problems` for the card that now speaks for them.

Every serialized value is an integer or a string. Coupling similarity and
concentration share are published as their integer operands — shared and union
commits, numerator and denominator — rather than as ratios, so a consumer
derives any ratio at whatever precision it wants and no floating-point value
appears anywhere in the object.

Terminal limits never remove JSON facts. The checked schema is
[`schemas/report-v4.schema.json`](schemas/report-v4.schema.json); no earlier
schema is emitted.

Source findings are ordered by rating, role class, hot state, count of signals
at that rating, total triggered signals, cognitive complexity, cyclomatic
complexity, logical lines, activity, path, and span. Role class places primary
source before non-primary source at equal rating, and non-primary source stays
visible below it rather than being removed. Nothing is filtered on role, so a
repository whose only rated debt is non-primary still names it as the worst
offender rather than reporting nothing. Hot state decides next, so among
production findings of the same rating the file being edited comes first. Hot
means a rated file whose touch count inside the selected history window reaches
the minimum touch count, five by default. Findings show unit kind and any
non-primary role.

Finding debt does not fail the command. Exit codes describe whether Smackdebt
could produce a report:

| Code | Meaning |
| ---: | --- |
| 0 | Report produced |
| 1 | Analysis could not produce a report |
| 2 | Invalid arguments or configuration |
| 3 | Gate baseline exceeded |

Common mistakes get one exact line on standard error, an empty standard
output, and no usage tail:

```console
smackdebt: path not found: does/not/exist
smackdebt: no source files found under: docs
smackdebt: not a source file: README.txt
smackdebt: Git ref not found: no-such-ref
smackdebt: --all cannot be used with --json
smackdebt: baseline not found: .smackdebt-baseline.tsv
```

## Gate

`smackdebt gate` ratchets debt against a committed baseline,
`.smackdebt-baseline.tsv` at the analyzed root by default; `--baseline`
selects another file. The baseline is a sorted, tab-separated table of High
and Watch counts per path and signal, and the gate ratchets exactly ten
time-invariant signals: `cognitive`, `cyclomatic`, `logical_lines`,
`nesting`, `parameters`, `file_size`, `container_size`, `package_cycle`,
`file_cycle`, and `stable_dependency`. History-derived signals such as change
coupling, knowledge concentration, and the two co-change patterns stay out of
the gate by rule: they move with wall-clock time, and a committed gate must not.
A report carrying leakage findings produces the same gate comparison as the same
tree read without history at all.

Any counter above its baseline is a regression: the gate names the row as
`worse` with the moved counter, states the totals, and exits with status 3.
Counters below the baseline are improvements, reported as `better` rows and
never applied to the file. An unchanged tree compares clean and exits 0.

`smackdebt gate --update` writes the observed debt as the new baseline and
exits 0. It accepts improvements and deliberate new debt alike; the gate never
tightens or rewrites the baseline on its own, so a clean check run never
touches the working tree. `smackdebt gate --json` writes the comparison as one
JSON object — schema version 1, described by `schemas/gate-v1.schema.json` —
whose regression and improvement rows each carry both counters' baseline and
observed values. This repository commits its own baseline and runs the gate as
part of its complete check.

## Configuration

Configuration is optional. Add `.smackdebt.toml` at the repository root when
the defaults do not fit the project:

```toml
history = "180d"
exclude = ["vendor/**", "fixtures/generated/**"]

[source_roles]
test = ["spec/**"]
example = ["examples/**"]
benchmark = ["benches/**"]
fixture = ["testdata/**"]
generated = ["src/client/generated.rs"]
vendored = ["public/javascripts/**"]

[thresholds.cognitive]
watch = 15
high = 25

[thresholds.cyclomatic]
watch = 11
high = 21

[thresholds.function_lines]
watch = 50
high = 100

[thresholds.file_lines]
watch = 400
high = 800

[thresholds.container_lines]
watch = 300
high = 600

[thresholds.nesting]
watch = 4
high = 7

[thresholds.parameters]
watch = 6
high = 9

[hotspots]
minimum_touches = 5
```

Command-line values override the project file. JSON is always unstyled and
undecorated, and an explicit `--color` cannot be combined with `--json`.

## Design

Read [ARCHITECTURE.md](ARCHITECTURE.md) for the domain model, data flow,
aggregation rules, Git behavior, and language extension path.

## Credits

Smackdebt uses tree-sitter and its language grammars for syntax trees. Smackdebt
owns the measurement rules, repository discovery, health policy, Git context,
aggregation, comparison, and reports.
