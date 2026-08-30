## Context

Smackdebt's presentation layer was designed around rows: one row per finding,
one row per relation, one row per history pair. That shape holds at repository
scope, where a truncation to three findings hides the volume, and collapses
everywhere else. At directory scope a 296-file selection prints roughly 1,270
unrated edge rows and the reader cannot find a single problem in them. The fix
is not a cap on the existing rows — a cap would hide arbitrary rows rather than
name problems. The fix is to change the unit of presentation from a measurement
to a problem.

Everything the cards say is already in the report. Analysis rates units,
detects cycles, computes fan-in and fan-out, finds hotspots, size findings,
coupling pairs, and concentration. Nobody has ever grouped those facts by the
thing they are about. That grouping — and the naming that comes with it — is
the whole change on the analysis side; the terminal side is a budget and a
renderer for the grouped result.

> **Constants under review**
>
> The following constants are proposed values awaiting user review during
> implementation; each is flagged where it lands, implemented as a named
> integer constant so review can move it in one place, and calibrated before
> the change is archived:
>
> - `god_file` arms (slice 3): `GOD_HIGH_FINDINGS = 3`,
>   `GOD_RATED_UNITS = 8`, `GOD_FAN_OUT = 10`.
> - `hub` arms (slice 3): `HUB_DEGREE = 8`, `HUB_MEDIAN_MULTIPLE = 4`.
> - Screen budget (slice 5): `SCREEN_BUDGET = 24` slots and the ladder
>   `[(6, 3), (8, 2), (12, 1), (24, 0)]`.
>
> `hotspot-analysis`'s minimum touch count and `verdict-policy`'s tier terms
> are not reopened here.

## Goals / Non-Goals

**Goals:**

- Make the default view answer "what are the problems here" at every scope in
  about one screen.
- Name each problem with a frozen id a machine consumer can key on and a
  sentence a human can act on.
- Keep every fact the report already owns available: nothing is deleted from
  JSON, and the terminal loses only rows that were never actionable.
- Keep serial and parallel output byte-identical and keep the ratchet baseline
  untouched.

**Non-Goals:**

- No new measurement, rating, threshold on a metric, or verdict input. Cards
  read existing tables.
- No new schema version. Version 4 gains members additively.
- No change to the frozen tier ids, tier sentences, worst-offender reason ids,
  or the accepted finding rank.
- No trajectory or trend tag in this change.
- No card vocabulary for diff mode in this change.

## Decisions

### Problem cards are clusters, not a new signal

A `ProblemCard` carries a pattern id, a rating, an anchor, ordered evidence, the
findings it claimed, and whether it is descriptive. Every one of those values is
read from a table the report already built, so clustering adds no measurement
and no rating. It runs once inside `ReportBuilder::finish()` after aggregation
and deliberately does **not** record an algorithm pass: the live work evidence
asserts exact algorithm pass counts per flow, and a grouping over already-built
tables is not new algorithmic work. Clustering takes borrowed slices rather than
a `Report`, so every detector is unit-testable without composing a report.

The report owns one globally ranked `problems` table. There is no per-scope card
list: output filters the ranked table by whether a card's anchor lies inside the
displayed scope, the same shape `stable_dependency_rows` already uses for
package rows. That keeps ordering built once and keeps scope selection a
predicate rather than a second sort.

### The pattern ids are frozen, and eight is the whole vocabulary

`god_file`, `hub`, `tangle`, `hot_mess`, `shotgun_pair`, `bus_risk`,
`unstable_dependency`, `measured`. Analysis owns the ids as a machine contract
the way it owns tier ids and worst-offender reason ids; output owns the human
names. `measured` is the fallback so that no verdict-affecting finding can
vanish from the terminal because no pattern recognized it — the fallback's head
is exactly today's `FINDINGS` row head, which is why the change loses no
information for a file that fits no named pattern.

### Claiming order is a policy, and it is total

`tangle` → `god_file` → `hub` → `hot_mess` → the per-table passes
(`shotgun_pair`, `bus_risk`, `unstable_dependency`) → `measured`. Each finding
is claimed at most once, audited the way the debt-diff selection is audited for
duplicate identities. Because every file-anchored pattern claims the file's
findings, at most one file-anchored card exists per file, which is the property
that kills today's "three rows, same file" noise.

The order encodes a judgment about what the reader should be told first: a file
that is in a cycle is a cycle problem; a file that does too much is an
overload problem before it is a fan-in problem; heat is the tiebreak of last
resort among file patterns.

### A tangle is one card per architecture finding — this is presentation only

Analysis already emits exactly one architecture finding per strongly connected
component, and that finding already carries a stable stacked witness. There is
no per-member or per-witness fan-out defect to fix. What changes is where the
witness prints: today it is a row under `ARCHITECTURE`, after this change it is
the evidence of one `tangle` card that also states the member count and the
hottest member. The spec deltas say exactly this, so nobody later reads them as
describing a bug that was never there.

### `god_file` needs a size-or-reach arm, not a finding count alone

`(high >= 3) OR (high >= 1 AND rated >= 8)`, **and** the file either carries a
size finding or has verdict-graph fan-out of at least 10. The second conjunct is
what makes the pattern mean "does too much" rather than "has bugs". Without it,
a small file with three High functions — a genuinely complex algorithm in one
place — would be named a god file, which is wrong and would push a real
overloaded file down the rank. With it, `god_file` requires both concentrated
debt and evidence of breadth: either the file is physically oversized or a lot
of the repository reaches through it.

`rated >= 8` is the risky arm. Vue single-file components produce at least two
units per file by construction, so a component directory can clear a unit-count
threshold that a Rust module never would. That arm is the first thing the
calibration task checks.

### Hub thresholds are package-relative, and the median is nearest-rank

`fan_in >= 8 AND (median_in(package) == 0 OR fan_in >= 4 * median_in(package))`,
and the same rule for fan-out. An absolute degree threshold alone is wrong
across repositories: eight importers is remarkable in a flat library and
unremarkable in a component tree. The median is computed once per package over
that package's files, using nearest-rank `sorted[(n - 1) / 2]`, so it is an
integer, needs no interpolation, is O(files) once, and — crucially — does not
depend on the selected scope. A scope-relative median would make the same file a
hub at one zoom level and not at another, and would break the invariant that
zooming changes which problems are shown rather than what the problems are.

File degree comes from `dependency_degree` over edges where
`DependencyEdge::enters_verdict_graph()` holds, so test, example, benchmark,
module-ownership, and recovered edges contribute nothing. That is the same
predicate the architecture verdict already uses, so a test file importing its
subject can never manufacture a hub, and the parent-child module imports that
dominate a Rust tree are excluded by construction.

### Descriptive cards keep healthy popular files out of the default view

A widely imported file is not automatically a problem — a well-factored utility
with high fan-in is the system working. A `hub` card whose file carries no rated
finding and no size finding is therefore **descriptive**: it exists in JSON and
under `--all`, carries no rating, and never appears in the default view or the
budget. This is the same "descriptive fact" idea the accepted specs already
apply to orphan files, degree, and instability.

### The budget counts slots, not rendered lines

`SCREEN_BUDGET = 24`. A card costs one slot plus one slot per shown evidence
line. The ladder is `[(6, 3), (8, 2), (12, 1), (24, 0)]` and the first rung
whose card count is at least the number of cards in the displayed scope applies;
every rung costs exactly 24 slots, so the budget is constant and only the
trade between breadth and depth moves. When a scope holds more cards than the
largest rung, the last rung applies and the cards beyond 24 are cut — that cut
*is* the budget, and `--top N` is the way to lift it.

Slots rather than rendered lines is a deliberate trade. Rows stack their facts
onto indented lines when they do not fit the resolved width, so at 50 columns a
24-slot section can render more than 24 lines and exceed one physical screen.
Budgeting rendered lines instead would make the *content* of the report depend
on terminal width: the committed 50-, 80-, 100-, and 120-column snapshots of one
invocation would state different facts, a reader could not compare two runs from
two terminals, and every existing width test would have to assert different
content per width. Width-independent content is worth more than a guarantee that
holds at 50 columns, and 50 columns is already the width where the accepted
specs accept stacking. So: identical facts at every width, one screen at the
widths people actually read reports in.

`--top N` selects the ladder rung that N would select and shows at most N
cards, so a larger N buys breadth by spending depth exactly as the default does.
`--all` shows every card, descriptive ones included, with full evidence, and
still shows no edge rows. A file scope shows every card of that file in full,
because a file holds few cards and drilling to a file is a request for detail.

### Edges leave the human terminal entirely

`relationship_rows` and the `relationships` flag are deleted;
`unmatched_import_rows` becomes gated on `--all` or a file scope. Today's
`detail` flag — "true whenever the scope is not the repository" — was the whole
mechanism that turned zooming into flooding, and it disappears.

This overturns one accepted sentence and one accepted spec clause, and both
reversals are stated in the deltas rather than left implicit: `terminal-output`
promised that a selected path retains relevant incoming and outgoing
debt-bearing relationships, and `architecture-analysis` promised that `--all`
and path drill show relevant relations. The accepted terminal rule that human
output omits raw dependency edges is the one that survives, because it is the
rule the field evidence supports; edge relationships now reach the reader as
card evidence (`17 files import this`) and reach a machine consumer complete
through the unchanged JSON relation tables.

Ruling carried into the deltas: unresolved and ambiguous relation rows print
when `--all` is supplied **or** the selected scope is a file. `--all` means "all
useful debt", and an unfollowed import at a file the user is looking at is
useful; at every other scope the grouped `WARNINGS` sentence, which already
states the total and its causes, is their whole terminal presence.

### The repository frame is analysis-owned bytes

A sub-scope verdict answers about that scope, which is right, and leaves the
reader without a sense of proportion. The verdict gains a share fact —
`42 of the repository's 136 high live here.` — computed by analysis and
rendered verbatim, mirroring `CoverageQualifier` exactly: analysis owns the
sentence so every consumer prints identical bytes, the fact never moves the
tier, and it is absent at the repository root where it would be a tautology.

### Trajectory is deferred, on purpose

A "getting worse" tag is the obvious next card feature and cannot be built
honestly yet. The ratchet gate deliberately excludes every history-derived
signal because they move with wall-clock time, and the codebase carries no
committed per-file history of its own ratings. Anything a card could say about
direction today would either be a wall-clock-sensitive claim — which would make
`just check` flaky, the exact failure mode the gate's signal list was designed
to prevent — or a re-reading of the coupling and concentration signals that
already have their own cards. The evidence vocabulary is an enum, so adding a
`Trend` variant later is purely additive to the model, to the schema, and to the
renderer's per-variant words.

### Additive v4, not v5

The `problems` table, `verdict.share`, and the size-finding position contract
are added to schema version 4. `earn-the-verdict` set the precedent by adding
the `nested_repository` diagnostic kind, coverage byte totals, and the verdict
qualifier to v4 rather than opening v5: version 4 is the contract until
something is *removed* or a member changes meaning, and nothing here does
either. The schema uses `additionalProperties: false` with `required` lists, so
the schema file and this spec change land in the same slice.

## Risks / Trade-offs

- **Threshold calibration is the real risk.** Vue single-file components inflate
  unit counts, so `rated >= 8` may name ordinary components as god files.
  Mitigation: a dedicated calibration task runs the built binary over a real Vue
  application and over this workspace at root, package, directory, and file
  scope, and the card sets are reviewed before the constants are frozen.
- **Single-package repositories degrade the hub median to a global median.**
  Acceptable — the median still describes the population the file lives in — but
  a multi-package fixture must exercise the package-relative path so the rule is
  actually covered.
- **Total snapshot churn is the review cost.** Terminal snapshots move twice.
  Mitigated by the slice split: slice 2 is terminal-only relief on today's
  model, slice 4 is JSON-only, and slice 5 rewrites the terminal on a baseline
  that is already quiet.
- **One cycle of split vocabulary.** Codebase mode speaks in cards while diff
  mode still speaks in sections. Deliberate and stated in the proposal;
  card-ifying diff needs its own comparison vocabulary and would double this
  change's snapshot churn.
- **A 50-column default view can exceed one physical screen** because slots are
  not lines. Accepted above as the price of width-independent content.
- **`--top N` changes meaning** from findings to cards. It is a middle level of
  detail either way, its conflicts with `--json` and `--all` are unchanged, and
  the README moves with it.

## Migration Plan

1. Author and strict-validate this change (slice 0).
2. Fix the multi-line dependency specifier (slice 1) — independent of the
   presentation work and near-zero snapshot impact, so it lands first.
3. Delete the edge rows and their gates (slice 2). This is the relief slice: it
   alone takes the reported 1,314-line view to roughly 50 lines, on the existing
   section model, with the affected acceptance tests inverted and four terminal
   snapshots regenerated one by one.
4. Build the card model and detectors in analysis (slice 3) and serialize them
   (slice 4). Both are invisible in the terminal, so the analysis and JSON
   reviews are separable.
5. Replace the sections with `PROBLEMS` and the budget (slice 5). Every terminal
   snapshot changes; each is regenerated per case.
6. Add the repository share end to end (slice 6).
7. Calibrate the constants on real repositories, then documentation and
   close-out (slice 7), confirming the ratchet baseline is unchanged with
   `just gate`.

Rollback is per-slice revert throughout; no slice writes a committed baseline or
a stored artifact that a revert would leave behind.
