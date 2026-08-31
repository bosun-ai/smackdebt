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
- [ ] 3.7 Add the `evolution-wide` workload profile — roughly two thousand files, fifty packages, forty commits with real cross-directory pairs, several provably unlinked pairs, and one bulk commit — wiring `PROFILES`, `GRAPH_PROFILES`, the `release-baselines.sh` profile loop from eight to nine, `EXPECTED_WORK`, `test_workload.py`, and a recorded baseline under `benchmarks/baselines/`.
- [ ] 3.8 Extend `scripts/performance/check-report.py` with the file-pair mirror block: bounds, lower index first, shared at most union, distance at least one, and no finding below the detector floors.
- [x] 3.9 Assert exact equality of inventory walks, reads, Git processes, parser visits, and algorithm passes with the pre-change values on every affected flow, and regenerate the JSON snapshots per case.

## 4. Change amplification

- [ ] 4.1 Accumulate the sparse per-directory histograms during the one history stream, with the observation value clamped at the maximum file count and one observation per commit per touched directory and its ancestors, reusing the directory tree built for pair accumulation rather than building a second one.
- [ ] 4.2 Add the `ChangeAmplification` value object with its materiality rule — commit floor, median floor, complete history — and chain it into the repository, package, and directory scope verdicts, leaving a file scope without one.
- [ ] 4.3 Render the sentence verbatim in the verdict block and serialize `verdict.amplification`, extending the checked schema in the same commit.
- [ ] 4.4 Add pure tests for the exact nearest-rank median, the ancestor de-duplication that counts a commit once per directory, the clamp, and each materiality boundary; prove the fact never moves the tier, its counts, or the worst offender.
- [ ] 4.5 Add exact acceptance at repository, package, and directory scope, the absence at a file scope, and a 50-column rendering, then regenerate the affected snapshots per case. Extend `bulk_commit_repository()` acceptance to prove the remaining half of the bulk-commit guard: the sweeping commit contributes exactly one amplification observation, so the fact is what it would have been without the guard.

## 5. Leakage detectors and cards

- [ ] 5.1 Add the change leakage module: the finding type with its two kinds, the pure `change_leakage(pairs, graph)` join at report finish, and the finding order of kind, distance descending, shared commits descending, then the two file identities.
- [ ] 5.2 Implement the leaky-interface rule over edges that enter the file dependency cycle graph, importer side only, with the distance floor, the support floor, and the distance-scaled similarity bar, creating exactly one finding per interface-and-follower pair and none for a mutual dependency.
- [ ] 5.3 Build the connection graph — every `uses` and `module_ownership` relation between primary trusted files — and the package closure matrix over it, which is the only package matrix this change builds and is deliberately not a verdict-graph closure, with a pure test proving an owning pair is connected in it and absent from the file dependency cycle graph.
- [ ] 5.4 Implement the hidden-coupling rule with the two-stage absence proof over that one graph: the package connection stage, then two budgeted probes, with an undecided answer producing no finding.
- [ ] 5.5 Append `LeakyInterface` and `HiddenCoupling` after `Measured` in the pattern enum, and prove that every pre-existing card keeps its pattern, claims, and rank position over the same report.
- [ ] 5.6 Implement hybrid claiming: a leakage finding belongs to its interface file or to the lower-indexed file of its pair, a file-anchored card claims the leakage findings of its file, `measured` fires only on unclaimed source and size findings so a leakage finding alone never triggers it, and the two tail patterns card only what is left.
- [ ] 5.7 Extend the audits: the claimable table set, the pattern list, the evidence kinds, the claimed-once audit, and the coverage audit over the change-leakage table.
- [ ] 5.8 Add the visibility arm that makes a card claiming a change-leakage finding `default`, with a test proving a healthy hub that leaks reaches the default view.
- [ ] 5.9 Add the exact evidence wordings and the new pattern names, rename the `shotgun_pair` human name to `packages change together`, and write the file-pair anchor as `<left> ↔ <right>`.
- [ ] 5.10 Serialize the `change_leakage_findings` table, the new claim and evidence kinds — the change-leakage index, the anchor reach, and the follower count — and the two pattern ids, extending the checked schema in the same commit.
- [ ] 5.11 Delete the README vocabulary ban on `change together without a dependency`, require the phrase instead, and record the reason where the assertion was written.
- [ ] 5.12 Update the edge-row invariant and its documentation for the co-change carve-out, and prove a card carrying two file paths still carries no reference count, relation kind, resolution outcome, or ownership wording.
- [ ] 5.13 Add the `change_leakage_repository()` fixture — a leaky interface three directories away, a hidden pair separated by the package stage, a same-directory pair, a test importer, and an owning pair that the connection graph joins — with exact acceptance for the two findings and the three absences.
- [ ] 5.14 Prove the strict launch: at most one new default card over the fixture, the one-screen budget holding at every scope, and a below-floor pair present in JSON and absent from the default view, the file scope, and `--all`.
- [ ] 5.15 Regenerate every affected terminal and JSON snapshot case by case, never as a batch, keeping the fifty-column display-width audit passing.

## 6. Calibration, documentation, and close-out

- [ ] 6.1 Calibrate against real repositories: run the release binary over this workspace at the repository root, `crates/analysis`, and `crates/output`, and over the private Fluyt repository at its root and one busy directory, each at a 90-day and a 365-day window; record every resulting card set in this file before any constant is frozen.
- [ ] 6.2 Freeze or adjust the constants from the recorded runs — the pair guards, the leakage floors and the similarity bar, the amplification clamp and floors, the reach and core materiality rules — updating the specs where a proposed value moved, and record zero findings on a quiet repository as a legitimate outcome rather than a reason to lower a floor.
- [ ] 6.3 Update the README: the two new patterns with their words and thresholds, the `packages change together` rename, the three verdict sentences and when each is absent, the new JSON tables and verdict members, the weak-pairs-are-JSON-only rule, and the statement that these signals never gate and never appear in a diff.
- [ ] 6.4 Update `ARCHITECTURE.md` for the change graph, the join at report finish, the closures and their bounds, and correct its stale claim that weak coupling is available through `--all`.
- [ ] 6.5 Note the stale performance `report_digest` values per precedent, confirm the committed ratchet baseline is unchanged with `just gate`, pass `openspec validate --all --strict` and the complete check, tick every task, and archive this change.
