## 1. Pure primitives

- [x] 1.1 Add the directory tree in analysis: directory identity, parent and depth vectors, the file-to-directory lookup, interned component names, the ancestor walk, and the integer distance `depth(a) + depth(b) - 2 * depth(lca)`, built once from the paths discovery already owns in time proportional to total path components.
- [x] 1.2 Add the reachability module: strongly connected component condensation with a reverse-topological bit-set closure, a largest-component size, a caller-bounded reach-in count, and a budgeted bidirectional path probe answering reaches, separate, or undecided.
- [x] 1.3 Move the nearest-rank median out of the problem module into a shared home and point both existing callers at it, changing no value.
- [x] 1.4 Add pure tests: distances of 0, 1, 2, and across two subtrees; diamond, cycle, and two-hundred-thousand-node chain reach; a probe that answers separate, one that answers reaches, and one that answers undecided exactly one node short of its budget; stack safety on the chain.
- [x] 1.5 Regenerate the analysis API snapshot for the new surface and review it.

## 2. Core size and propagation reach

- [x] 2.1 Retain the file pairs and the file strongly connected components the architecture build already computes, and carry them out of the build so the report can hold them.
- [x] 2.2 Compute each package's reach-in count over the verdict package graph and expose `reach_in` on the package-graph measurement.
- [x] 2.3 Compute the file closure for every package eagerly inside `build_architecture`, one row per package below the closure node limit with one transient bit set live at a time, skipping above the limit and disclosing the skip as a `propagation_skipped` diagnostic; add a work-counter test proving that rendering a package scope from the finished report performs no closure.
- [x] 2.4 Compute exact reach for the bounded candidate set — cycle members and hub-degree files, ordered by fan-in descending then path, cut at the candidate limit — and store it as the candidate reach table.
- [x] 2.5 Add the `PropagationReach` and `CoreSize` value objects beside the verdict share with smart constructors that return nothing below their materiality rules, chain them into the scope verdict, and prove with a pure test that neither moves the tier, its counts, or the worst offender.
- [x] 2.6 Render the two sentences verbatim in the verdict block after the share line, and serialize `verdict.reach`, `verdict.core_size`, the `package_closures` and `file_reach` tables, and `reach_in`, extending the checked schema in the same commit.
- [x] 2.7 Add the `propagation_repository()` and `core_repository()` fixtures with their absence cases — a one-package repository stating no reach, a three-of-two-hundred core stating no sentence — and exact acceptance at root and package scope, including a 50-column rendering of each sentence.
- [x] 2.8 Regenerate the affected terminal and JSON snapshots one case at a time and record the expected diff shape in the commit body.

## 3. The file co-change accumulator

- [x] 3.1 Add `HistoryChangeFact::enters_change_graph()` as primary role and trusted parse, named to mirror the dependency-edge verdict predicate, with a pure test proving a test-role change is excluded.
- [x] 3.2 Add the file change coupling accumulator as a member of the evolution accumulator: per-commit distinct change-graph files, the bulk-commit guard, cross-directory pairs only, its own per-file commit count, and union derived from that count.
- [x] 3.3 Apply the retention floors and the pair storage limit, counting bulk commits and declined pairs, and add the history coverage builder that discloses both without touching the existing constructor.
- [x] 3.4 Serialize the `file_change_coupling` table with the lower file index first, integer shared and union commits, and the directory distance, extending the checked schema in the same commit.
- [x] 3.5 Add boundary tests: a pair one commit below the retention floor, a pair one permille below it, a same-directory pair storing nothing, a test file and its subject producing no pair, and a pair whose union excludes a bulk commit.
- [x] 3.6 Add the `bulk_commit_repository()` fixture and prove end to end that a thirty-file commit yields one bulk commit, no pair, and unchanged churn, touches, package coupling, and concentration. The guard's amplification half is proven in 4.5, which has no fact to read until amplification exists.
- [x] 3.7 Add the `evolution-wide` workload profile — roughly two thousand files, fifty packages, forty commits with real cross-directory pairs, several provably unlinked pairs, and one bulk commit — wiring `PROFILES`, `GRAPH_PROFILES`, the `release-baselines.sh` profile loop from eight to nine, `EXPECTED_WORK`, `test_workload.py`, and a recorded baseline under `benchmarks/baselines/`. The timing record is the one half deferred: every committed baseline shares one workspace revision, so a lone new record would fail `check-baselines.py`, and this change deliberately re-records no release evidence. The profile is in the release loop and its record lands with the next `just release-baselines`, which the delta now states.
- [x] 3.8 Extend `scripts/performance/check-report.py` with the file-pair mirror block: bounds, lower index first, shared at most union, distance at least one, and no finding below the detector floors.
- [x] 3.9 Assert exact equality of inventory walks, reads, Git processes, parser visits, and algorithm passes with the pre-change values on every affected flow, and regenerate the JSON snapshots per case.

## 4. Change amplification

- [x] 4.1 Accumulate the sparse per-directory histograms during the one history stream, with the observation value clamped at the maximum file count and one observation per commit per touched directory and its ancestors, reusing the directory tree built for pair accumulation rather than building a second one.
- [x] 4.2 Add the `ChangeAmplification` value object with its materiality rule — commit floor, median floor, complete history — and chain it into the repository, package, and directory scope verdicts, leaving a file scope without one.
- [x] 4.3 Render the sentence verbatim in the verdict block and serialize `verdict.amplification`, extending the checked schema in the same commit.
- [x] 4.4 Add pure tests for the exact nearest-rank median, the ancestor de-duplication that counts a commit once per directory, the clamp, and each materiality boundary; prove the fact never moves the tier, its counts, or the worst offender.
- [x] 4.5 Add exact acceptance at repository, package, and directory scope, the absence at a file scope, and a 50-column rendering, then regenerate the affected snapshots per case. Extend `bulk_commit_repository()` acceptance to prove the remaining half of the bulk-commit guard: the sweeping commit contributes exactly one amplification observation, so the fact is what it would have been without the guard.

## 5. Leakage detectors and cards

- [x] 5.1 Add the change leakage module: the finding type with its two kinds, the pure `change_leakage(pairs, graph)` join at report finish, and the finding order of kind, distance descending, shared commits descending, then the two file identities.
- [x] 5.2 Implement the leaky-interface rule over edges that enter the file dependency cycle graph, importer side only, with the distance floor, the support floor, and the distance-scaled similarity bar, creating exactly one finding per interface-and-follower pair and none for a mutual dependency.
- [x] 5.3 Build the connection graph — every `uses` and `module_ownership` relation between primary trusted files — and the package closure matrix over it, which is the only package matrix this change builds and is deliberately not a verdict-graph closure, with a pure test proving an owning pair is connected in it and absent from the file dependency cycle graph.
- [x] 5.4 Implement the hidden-coupling rule with the two-stage absence proof over that one graph: the package connection stage, then a budgeted walk from one end of the pair and, when it exhausts its budget, from the other, with a pair both walks leave undecided producing no finding.
- [x] 5.5 Append `LeakyInterface` and `HiddenCoupling` after `Measured` in the pattern enum, and prove that every pre-existing card keeps its pattern, claims, and rank position over the same report.
- [x] 5.6 Implement hybrid claiming: a leakage finding belongs to its interface file or to the lower-indexed file of its pair, a file-anchored card claims the leakage findings of its file, `measured` fires only on unclaimed source and size findings so a leakage finding alone never triggers it, and the two tail patterns card only what is left.
- [x] 5.7 Extend the audits: the claimable table set, the pattern list, the evidence kinds, the claimed-once audit, and the coverage audit over the change-leakage table.
- [x] 5.8 Add the visibility arm that makes a card claiming a change-leakage finding `default`, with a test proving a healthy hub that leaks reaches the default view.
- [x] 5.9 Add the exact evidence wordings and the new pattern names, rename the `shotgun_pair` human name to `packages change together`, and write the file-pair anchor as `<left> ↔ <right>`.
- [x] 5.10 Serialize the `change_leakage_findings` table, the new claim and evidence kinds — the change-leakage index, the anchor reach, and the follower count — and the two pattern ids, extending the checked schema in the same commit.
- [x] 5.11 Delete the README vocabulary ban on `change together without a dependency`, require the phrase instead, and record the reason where the assertion was written.
- [x] 5.12 Update the edge-row invariant and its documentation for the co-change carve-out, and prove a card carrying two file paths still carries no reference count, relation kind, resolution outcome, or ownership wording.
- [x] 5.13 Add the `change_leakage_repository()` fixture — a leaky interface three directories away, a hidden pair separated by the package stage, a same-directory pair, a test importer, and an owning pair that the connection graph joins — with exact acceptance for the two findings and the three absences.
- [x] 5.14 Prove the strict launch: at most one new default card over the fixture, the one-screen budget holding at every scope, and a below-floor pair present in JSON and absent from the default view, the file scope, and `--all`.
- [x] 5.15 Regenerate every affected terminal and JSON snapshot case by case, never as a batch, keeping the fifty-column display-width audit passing.

## 6. Calibration, documentation, and close-out

- [x] 6.1 Calibrate against real repositories: run the release binary over this workspace at the repository root, `crates/analysis`, and `crates/output`, and over the private Fluyt repository at its root and one busy directory, each at a 90-day and a 365-day window; record every resulting card set in this file before any constant is frozen.
- [x] 6.2 Freeze or adjust the constants from the recorded runs — the pair guards, the leakage floors and the similarity bar, the amplification clamp and floors, the reach and core materiality rules — updating the specs where a proposed value moved, and record zero findings on a quiet repository as a legitimate outcome rather than a reason to lower a floor.
- [x] 6.3 Update the README: the two new patterns with their words and thresholds, the `packages change together` rename, the three verdict sentences and when each is absent, the new JSON tables and verdict members, the weak-pairs-are-JSON-only rule, and the statement that these signals never gate and never appear in a diff.
- [x] 6.4 Update `ARCHITECTURE.md` for the change graph, the join at report finish, the closures and their bounds, and correct its stale claim that weak coupling is available through `--all`.
- [x] 6.5 Note the stale performance `report_digest` values per precedent, confirm the committed ratchet baseline moves by exactly the one deliberate row with `just gate`, pass `openspec validate --all --strict` and the complete check, and tick every task. The change is **not** archived: the wave is reviewed as a whole first.

### 6.1 Calibration record

Release binary, `target/release/smackdebt`, at the constants below. Every run
was repeated before and after the two rule changes 6.2 records; the counts here
are the shipped behavior.

**This workspace** (170 commits in the 90-day window, 118 of them eligible;
history complete at both windows, so 90d and 365d read the same commits and
produce identical tables):

| Scope | Window | Head sentences beyond the tier | Retained pairs | Findings | Leakage cards |
| --- | --- | --- | ---: | ---: | ---: |
| repository root | 90d | `A change in one package can reach 6 of 12 packages.` · `9 of 86 files sit in one dependency cycle.` | 60 | 0 | 0 |
| repository root | 365d | the same two | 60 | 0 | 0 |
| `crates/analysis` | 90d | `4 of the repository's 20 high live here.` · `A change here can reach 17 of 36 files in this package.` · `A typical change here touches 4 files.` | 60 | 0 | 0 |
| `crates/analysis` | 365d | the same three | 60 | 0 | 0 |
| `crates/output` | 90d | `1 of the repository's 20 high live here.` · `A typical change here touches 5 files.` | 60 | 0 | 0 |
| `crates/output` | 365d | the same two | 60 | 0 | 0 |

**Fluyt** (private; 189 commits in 90 days, 1,087 in 365):

| Scope | Window | Head sentences beyond the tier | Retained pairs | Findings | Leakage cards |
| --- | --- | --- | ---: | ---: | ---: |
| repository root | 90d | `A change in one package can reach 6 of 16 packages.` · `A typical change here touches 4 files.` | 8 | 0 | 0 |
| repository root | 365d | `A change in one package can reach 6 of 16 packages.` · `A typical change here touches 3 files.` | 361 | 12 hidden | 4, `--all` only |
| `bow/src/components` | 90d | `21 of the repository's 138 high live here.` · `A typical change here touches 8 files.` | 8 | 0 | 0 |
| `bow/src/components` | 365d | `21 of the repository's 138 high live here.` · `A typical change here touches 7 files.` | 361 | 12 hidden | 0 at this scope |

Fluyt's twelve findings at 365 days are the strongest evidence the detectors
produced: eight of them pair a Vue/TypeScript front end file with the Ruby API
endpoint it calls over HTTP, five to eight directories apart, with no dependency
either way — `bow/src/api/identity.ts ↔ stern/app/api/identity/v1/auth.rb
changed together in 5 of 19 commits · 26% · no dependency either way · 8
directories away`. Four of them reach standalone cards, all `default` and all
ranked below the one-screen budget of a 493-card repository, so they appear
under `--all`.

**Strict launch.** Zero new default-view items on either repository at every
calibrated scope: the target was one to two. The fixture
`change_leakage_repository()` still proves one new default card, so the path
from finding to default view is exercised where it can be pinned.

### 6.2 Constant decisions

Every constant keeps its proposed value. Two rules changed, both because the
calibration runs showed a true statement that no reader can act on.

| Constant | Value | Decision |
| --- | ---: | --- |
| `BULK_COMMIT_FILES` | 25 | Frozen. Fluyt declines 15 of 189 commits at 90 days and 46 of 1,087 at 365; this workspace declines 2 of 170. Both are the sweeping commits the guard exists for, and both repositories still produce every finding they have. |
| `RETAINED_FILE_PAIR_SHARED_COMMITS` | 3 | Frozen. It keeps the table at 60 rows here and 361 on Fluyt — inspectable in JSON, far below any storage concern. |
| `RETAINED_FILE_PAIR_SIMILARITY_PERMILLE` | 100 | Frozen; no repository came near the limit from either side. |
| `RETAINED_FILE_PAIR_LIMIT` | 1,000,000 | Frozen. `declined_pairs` is 0 on both repositories at both windows and on the wide workload. |
| `LEAKAGE_MIN_DISTANCE` | 2 | Frozen. Every finding on both repositories sits at distance 2 or more by a wide margin (Fluyt's cluster is at 5 to 8). |
| `LEAKAGE_SHARED_COMMITS` | 5 | Frozen. Lowering it is what a quiet repository tempts you to do; this workspace at 90 days is a legitimate zero and the plan says so. Fluyt's own 90-day zero is 189 commits with only 8 retained pairs, none reaching five shared commits — thin history, not a bad floor. |
| similarity bar (400 / 50 / 200 permille) | as proposed | Frozen. Fluyt's findings run from 23% at distance 8 to 67% at distance 2, and the low end clears the bar only because distance lowered it: at a flat 40% bar seven of the twelve would vanish, including every front-end-to-API pair. |
| `PATH_PROBE_NODES` | 4,096 | Frozen. No pair on either repository was left undecided: every stage-two answer came from a component that fits. |
| `AMPLIFICATION_MIN_COMMITS` / `MIN_MEDIAN` / `MAX_FILES` | 10 / 3 / 1,000 | Frozen. The median floor is what keeps this workspace's root silent: 45 of its 106 windowed commits touch exactly one change-graph file and the repository-wide median is 2, while `crates/analysis` reads 4 and `crates/output` 5. A scope where changes really do arrive in fours says so and a repository of one-file commits does not, which is the rule working. |
| `PACKAGE_REACH_FILES` | 20 | Frozen. It leaves this workspace one closure row out of twelve packages and Fluyt seven out of sixteen — the packages large enough for the number to mean anything. |
| `ROOT_REACH_PACKAGES` / `ROOT_REACH_REACHED` | 3 / 2 | Frozen; both repositories state the root sentence and neither is near the floor. |
| `CORE_SIZE_FILES` / `CORE_SIZE_PERCENT` | 5 / 2 | Frozen. This workspace states `9 of 86` (10%); Fluyt states nothing, because its largest file cycle is below the floor. One repository saying it and the other not is the materiality rule working, not a threshold to lower. |
| `CLOSURE_NODE_LIMIT` | 4,096 | Frozen. Fluyt's largest package holds 521 graph files, an eighth of the limit, and no `propagation_skipped` diagnostic appeared on either repository. |
| `REACH_CANDIDATE_LIMIT` | 64 | Frozen. This workspace fills 23 of it and Fluyt 58, so the cut has never yet decided anything, and the bound still holds. |

**Rule change 1 — a conventional entry file is never a leaking interface.**
Before this rule, every `leaky_interface` finding on both repositories named a
wiring module: `crates/analysis/src/lib.rs` followed by `project.rs` (20 of 57),
`output.rs` (19 of 58), and `json.rs` (15 of 40); `crates/project/src/lib.rs`
followed by `crates/cli/src/app.rs` (5 of 14); and on Fluyt
`quak/quak-core/src/tools/mod.rs` followed by its registry (11 of 16) and
`quak/quak-core/src/agents/mod.rs` (6 of 6). Six of six. Each of those files is
a list of `mod` declarations and re-exports — `tools/mod.rs` is 68 lines of
exactly that and `agents/mod.rs` is 3 — so the finding says an export was added
and used, which is one edit, not an abstraction leaking. The rule reuses the
accepted `ENTRY_FILENAMES` list the orphan rule already publishes, applies to
the interface side only (the claim is about the file accused of leaking), and
leaves the pair with its dependency, so nothing falls through to the hidden
rule. Effect: this workspace 4 findings → 0, Fluyt at 365 days 14 → 12 and 6
cards → 4. Amended in `specs/change-leakage/spec.md` with a scenario, proved by
`an_entry_file_is_never_named_as_the_interface_whose_importers_follow_it`, and
documented in the README.

**Rule change 2 — a reach of zero is not a fact.** Fluyt printed `a change here
reaches 0 files` nine times at its root under `--all` and four times at
`bow/src/components`, on `hub` cards that fired on fan-out alone: a file that
imports twenty-two others and is imported by none is a reach candidate whose
reach is zero. The line states nothing, and this product leaves an immaterial
fact absent rather than printing it. The candidate table keeps the row; the card
states no line. Amended in `specs/problem-clustering/spec.md` with a scenario
and proved inside
`a_hub_states_its_exact_reach_after_the_degree_that_made_it_fire`.

**Considered and not implemented — suppressing a hidden card under its package
pair.** On the `history-strength-order` fixture the hidden card `b/main.js ↔
c/main.js` repeats the operands of the `packages change together · b ↔ c` card
one level up, because each of those packages holds exactly one file. On real
repositories the two never coincide: Fluyt carries `bow ↔ stern` at 119 of 389
commits and four file pairs at 5 to 8 of 14 to 19 — different subjects,
different numbers, and the file pair is the more specific and more actionable of
the two. Suppressing the file card would delete the best output the detectors
produced on the only repository that produces any. The clustering contract is
that each *finding* is claimed once, and it holds. Recorded in `design.md` with
the alternative — folding a pair whose operands match its package pair into that
card as evidence — left as a future option rather than a rule invented at
calibration time.

**Recorded, not changed — the graph-files denominator.** `A change here can
reach 17 of 36 files in this package.` counts the files the dependency graph is
built over, not the files the package holds: `crates/analysis` holds 41 (36 in
the graph), and on Fluyt `bow` holds 664 against 425, `stern` 707 against 521,
and `quak/quak-manifests` 50 against 26 — a gap running from a tenth to a half.
The population is deliberate and shared by both halves of the fraction, which is
what makes the fraction mean anything, and it is the population every other
dependency number in the report already counts. The sentence's own words
nonetheless claim more than that, so the population is now stated in
`specs/verdict-policy/spec.md` and in the README rather than left implicit. The
alternative — naming the population inside the sentence — would put a new
adjective into a frozen head sentence at the last slice, and the wording is a
locked user decision, so it is raised for review rather than taken here.

**Observed, not changed — fan-in and reach are counted over different graphs.**
On `crates/analysis/src/lib.rs` the same card reads `32 files import this` and
`a change here reaches 22 files`, which looks like a contradiction. Fan-in
counts the verdict graph; reach walks the file cycle graph, which excludes the
ten same-crate imports that accompany a `mod` declaration as wiring. Both
numbers are right for their own question and the smaller one follows the larger
on the card. Moving reach onto the verdict graph would change core size,
closures, and the candidate set that slices 1 and 2 pinned, so it is recorded
here for review rather than done at calibration time. On this workspace the card
is no longer produced at all, because the file is an entry file.

### 6.x Performance and close-out record

- **Diff-mode cost of computed-but-unstated work at `evolution-wide`.** The base
  commit `6dcdfd1` and this branch, both release builds, over the wide workload
  with forty files changed in the worktree: allocations 352,945 → 384,328
  (+31,383, +8.9%) and transient bytes 21,420,106 → 24,290,217 (+2,870,111,
  +13.4%). A diff report states none of it — no pair, no histogram, no closure,
  no reach, no core size — and `build_architecture` runs twice, which is why the
  diff pays nearly twice what the same workload's codebase run pays for facts it
  does state (247,331 vs 231,292 allocations, +6.9%). In wall time the
  difference is under the noise floor: two rounds of seven and fifteen samples
  put the branch between 20 ms faster and 9 ms slower on a ~320 ms diff run.
- **`evolution-dense` is a two-bulk-commit workload**, not a one-bulk-commit
  workload: both of its commits touch a hundred files. The delta scenario said
  one; it now says what the profile does, and the check asserts it.
- **`EXPECTED_WORK` was stale for every Git-bearing profile.** `evolution-dense`
  expected 308 inventory visits and produces 304, `small-diff` 112 against 108,
  and `large-dependency-diff` 1208 against 1204 — each four low, because
  discovery began honoring `.git/info/exclude` when it moved onto the ignore
  crate and every Git-bearing workload writes one naming four bookkeeping files.
  `check-report.py` runs only inside the baseline flow, so nothing had caught it.
  Corrected with the reason recorded beside the table, and every profile now
  passes the complete correctness-checked flow.
- **The evolutionary-finding check read a member that does not exist.** It
  compared `row["similarity"]` against 0.2, and the report publishes integer
  operands only, so any workload that produced an evolutionary finding crashed
  the checker. `evolution-wide` is the first that does. Rewritten as the integer
  comparison the report's own rule states.
- **Generated workloads now disable background Git maintenance.** A maintenance
  run racing the forty commits that build `evolution-wide` left the object store
  with a missing blob twice in a row, which `git fsck` confirmed; the workload
  became nondeterministic through no fault of the analysis.
- **Committed `report_digest` values are stale**, as they were for
  `earn-the-verdict` and `make-problems-legible`: this change moves report bytes
  and re-records no release evidence.
  `scripts/performance/check-baselines.py` validates committed records only, so
  `just performance-tests` stays green.
- **The ratchet baseline moves by exactly one deliberate row.** `just gate`
  reported `0 regressions · 1 improvement` — `crates/output/src/json.rs ·
  cyclomatic · watch 1 → 0`, earned by the evidence-serialization split in slice
  5 — and that improvement is ratcheted in with `gate --update` so the slack
  closes. No other row moved.
