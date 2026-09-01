## Why

Smackdebt now names problems. It still cannot answer the question a reader asks
immediately after reading one: **how far does a change here reach?**

Everything needed to answer it is already in the report and already paid for.
One streamed history yields one change fact per changed file per commit. The
architecture build already materializes the file dependency pairs and the
complete file strongly connected components, then drops them. Nothing joins the
two halves, so the report can state that a file has 41 importers, and separately
that two packages change together, but never that *this file's importers change
with it* — which is the observable signature of an interface that leaks its
internals. The reader has to hold the graph in one hand and the history in the
other and do the join by hand.

The second gap is that the report states no system-level number at all. A reader
cannot tell whether a change is contained or whether it can reach two thirds of
the repository, cannot tell how large the tangled core is relative to the whole
codebase, and cannot tell what a typical change here actually costs. Those are
the numbers that make an architecture argument, and they are three integers away
from facts the report already holds.

Both families are joins over existing data. Answering them costs no inventory
walk, no source read, no Git process, and no parser visit — it costs retention
and integer arithmetic inside passes that already run.

## What Changes

- **Two co-change ∧ dependency joins become findings.** A *leaky interface* is a
  file whose importers change with it across a directory boundary: the
  dependency exists and the change still propagates through it, so the interface
  is not doing its job. *Hidden coupling* is a pair of files that change together
  with **provably** no dependency of any kind between them in either direction,
  imports and module wiring alike — the proof is a two-stage absence check over
  one deliberately wide connection graph, and an inconclusive probe produces no
  finding, because absence is proved rather than inferred. Both live in one
  finding table with two kinds, rated Watch, following the `ArchitectureFinding`
  precedent.
- **Distance is severity.** Both detectors require the two files to sit in
  different directories, and the directory distance between them lowers the
  similarity bar an integer step at a time: two files four directories apart need
  weaker agreement to be interesting than two files in sibling folders, because
  distance is what makes co-change surprising.
- **Three system numbers reach the report.** *Propagation reach* answers
  how many packages one package's change can reach at the repository root and how
  many files one change can reach inside the selected package; *core size*
  answers how much of the repository sits in one file dependency cycle; *change
  amplification* answers how many files a typical change here touches. Reach is
  shown only beside the package or file that produces it, core size only beside
  the cycle it measures, and amplification stays machine-only until it can name
  a narrower place to inspect. All three are **stated only**: they never move a
  tier, a count, or the worst offender.
- **Hybrid surfacing, not more cards.** A file's leakage findings become evidence
  lines on the card that already names that file. A standalone card exists only
  when nothing else names the file or the pair. Propagation reach reaches a
  reader as one evidence line on a `hub` or a `tangle` card, where the reader is
  already looking at the thing that spreads.
- **A strict launch.** Every constant is a proposed value under review, the
  thresholds are set so that this repository and one private repository each gain
  at most one or two new default-view items, and every weaker observation stays
  in JSON where no human view — `--all` included — states it. A calibration task
  runs the release binary over real repositories before the constants are
  frozen, and zero leakage findings on a quiet repository is a legitimate
  outcome.
- **The pattern vocabulary grows from eight ids to ten.** `leaky_interface` and
  `hidden_coupling` are appended after `measured`, so claiming order — which is
  declaration order — leaves every existing claim byte-identical.
  `shotgun_pair`'s human name becomes `packages change together`, because a
  file-pair pattern now stands beside it and `changes together` no longer says
  which.
- **A co-change finding may name the two files it is about.** The accepted rule
  that no human view prints a dependency edge as a row keeps its force and gains
  an explicit carve-out: a finding's subject is the identity of the thing
  measured, not a graph row. This is the same principle that already lets a cycle
  witness print file paths and an `unstable_dependency` card print a package
  pair.
- **The gate does not move.** No signal is added, no threshold changes, and a
  report carrying leakage findings, a core-size fact, and a reach fact produces a
  gate snapshot identical to the same tree analyzed without history. The future
  shape of a `core_size` gate row is recorded in `design.md` and deliberately not
  specified as accepted behavior.
- **A diff explains architecture movement.** Reach, core size, and change
  leakage compare the current tree with the selected ref when both graph sides
  have sufficient evidence. The comparison names a package, file, cycle, or
  pair and counts in the existing architecture family. Amplification remains a
  codebase-only history fact.
- **Graph claims require sufficient evidence.** Package-local TypeScript and
  JavaScript aliases are resolved from the source package's configuration.
  Parse failures, unresolved or ambiguous internal references, and unusable
  configuration suppress affected human claims while machine diagnostics retain
  the reason.

## Capabilities

### New Capabilities

- `change-leakage`: the co-change model, the change-graph role filter and why
  the existing history predicate is wrong for it, the bulk-commit guard and its
  disclosure, retention thresholds against finding thresholds, the importer-side
  leaky-interface rule, the two-stage path proof with its undecided arm, the
  change-amplification median, and the rule that none of it moves a verdict.

### Modified Capabilities

- `architecture-analysis`: propagation reach as scoped closures, core size as
  the largest file strongly connected component, and the carve-out that lets a
  co-change finding name its two files.
- `verdict-policy`: three stated-only facts — reach, core size, and change
  amplification — with frozen analysis-owned sentences that never move the tier,
  its counts, or the worst offender.
- `evolutionary-analysis`: file change coupling and change amplification ride
  the one streamed history process and the selected window exactly as package
  coupling does, and history coverage discloses the commits and pairs the guards
  declined.
- `problem-clustering`: ten frozen pattern ids, two new patterns at the tail of
  claiming order, the hybrid rule that prefers evidence on an existing card to a
  new card, the extended claimable-table set, and reach as ordered evidence.
- `report-schema-v4`: additive tables for file change coupling, change-leakage
  findings, package closures, and candidate file reach; additive verdict members
  for the three system facts; two new pattern ids and their evidence kinds.
- `terminal-output`: the new pattern names and the `shotgun_pair` rename, the
  file-pair anchor, the new evidence wordings, the verdict-head sentences, and
  the co-change carve-out to the no-edge-rows rule.
- `analysis-performance`: bounded work for every new computation, the new
  `evolution-wide` workload profile, and exact unchanged work counts.
- `debt-ratchet`: no signal is added; one scenario proves the gate snapshot is
  identical with and without the new analytics.
- `end-to-end-evidence`: black-box evidence for each detector, the bulk guard,
  the system sentences and their absence cases, the strict-launch card budget,
  and the sanctioned return of one previously banned README phrase.
- `product-documentation`: the README documents the two new patterns and their
  thresholds, the three head sentences, and the new JSON tables.

### Referenced Without Change

- `hotspot-analysis`: the accepted finding rank and the hotspot rule are reused
  by reference; the problem rank is unchanged and this change restates neither.
- `progressive-exploration`: the one-screen budget and its ladder are unchanged.
  The strict launch is proven against them — the budget test shows the ladder
  still holds with the new cards present — rather than by changing a rung.

## Impact

- Affects `smackdebt-analysis` (four new modules: directory tree, reachability,
  file change coupling, change amplification, plus the leakage join and three
  verdict value objects), `smackdebt-project` (retain what
  `build_architecture` already computes, feed the new accumulator from the one
  history stream, run the join where the report is finished), and
  `smackdebt-output` (new evidence wordings, pattern names, and verdict rows).
  `smackdebt-languages`, `smackdebt-discovery`, `smackdebt-git`, and the CLI are
  untouched.
- `schemas/report-v4.schema.json` gains four tables and three verdict members.
  Version 4 stays the contract and gains members additively, following the
  precedent set when `earn-the-verdict` added coverage bytes and the verdict
  qualifier and `make-problems-legible` added the problems table.
- Terminal snapshots churn in slice 2 (verdict-head sentences) and slice 5
  (cards and evidence); JSON snapshots churn in slices 2, 3, 4, and 5. The slice
  order is chosen so each churn lands on an otherwise quiet baseline and every
  regenerated snapshot is reviewed per case, never as a batch.
- The ratchet baseline must not move this round, with one deliberate exception
  taken at the end: new analysis modules are sized and structured to stay under
  the accepted file and container thresholds, and every regression the gate
  reported during the wave was restructured away rather than accepted. The
  closing slice ratchets in `crates/project/src/project.rs · nesting · watch
  4 → 3`, earned by the architecture-composition cleanup, so that part of the
  slack the baseline still allowed closes.
- Live work counts are load-bearing: every new computation rides an existing
  pass — the history stream callback, the architecture build, and report finish —
  and no new algorithm pass is recorded, so the exact `algorithm_passes`
  assertions in acceptance and in the performance report check stay unchanged.
- A new public workload profile `evolution-wide` is added beside the existing
  profiles so the pair accumulator, the closures, and the path probes are
  measured on a workload that actually exercises them; `evolution-dense` becomes
  the bulk-guard proof. Its timing record is written by the next
  `just release-baselines`, whose profile loop now includes it: every committed
  baseline shares one workspace revision, so a lone new record would fail
  `check-baselines.py`, and this change re-records no release evidence. Its
  correctness half — the exact work counts, the pair table, the finding floors,
  and the deliberate history shape — is checked on every run of the flow and is
  green for all nine profiles.
- `ARCHITECTURE.md` is updated under its existing accepted
  `architecture-documentation` requirements, which already require it to describe
  the analysis and presentation boundary; no delta is needed there. Its stale
  claim that weak coupling is available through `--all` is corrected in the same
  pass.
- Committed performance-baseline `report_digest` values go stale until a future
  `just release-baselines`, as they did for `earn-the-verdict` and
  `make-problems-legible`; `scripts/performance/check-baselines.py` validates
  committed records only, so `just performance-tests` stays green.
- Three latent defects in the performance harness surfaced when the new profile
  first ran the complete correctness-checked flow, and each is fixed with its
  reason recorded: `EXPECTED_WORK` had been four inventory visits high for every
  Git-bearing profile since discovery began honoring `.git/info/exclude`; the
  evolutionary-finding check read a `similarity` member the report does not
  publish, so any workload with an evolutionary finding crashed it; and
  generated workloads now disable background Git maintenance, which had twice
  raced the commits building the wide profile and left its object store with a
  missing blob.
