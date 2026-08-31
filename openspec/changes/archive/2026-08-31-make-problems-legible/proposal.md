## Why

Smackdebt answers "how bad is it" well and "what are the problems" badly. Run
over a real application, the report at repository scope prints 47 useful lines.
The same tool at directory scope over 296 files prints 1,314 lines — roughly
1,270 of them one unrated, unranked row per file-to-file import edge, such as
`X.test.ts → X.vue · 1 import · test`. The report's own `next:` hint leads to a
1,843-line view, and `--top` caps only `FINDINGS`, so nothing the user can type
makes the output smaller. Zooming in makes the report worse, which is the exact
opposite of what progressive exploration promises.

Volume is not the whole failure. Even inside the useful lines the reader cannot
see the shape of the debt:

- One file with three High findings prints as three separate rows, so the
  reader counts rows instead of recognizing one overloaded file.
- The architecture section is cut to the first three findings by an unranked
  truncation, so a Watch file cycle can hide a High package cycle.
- A cycle's members and a hot complex file are stated as isolated facts that
  the reader has to reassemble into a named problem.

The root causes are in the product, not only the code: relationship rows switch
on for every non-repository scope with no cap and no rating, and the accepted
architecture spec bakes it in — `--all` and path drill "SHALL show relevant
incoming, outgoing, unresolved, ambiguous, advisory, and module-ownership
relations", where at directory scope "relevant" degenerates to "everything".
That accepted sentence also contradicts the accepted terminal rule that human
output omits raw dependency edges; this change resolves the contradiction in
favor of the terminal rule.

One separate defect is fixed alongside: an unresolved-import diagnostic can
carry an entire multi-line import statement as its target, so a template-literal
dynamic import prints raw source, newlines included, into the terminal.

## What Changes

- **Problem cards replace finding and edge rows.** Analysis clusters the
  findings it already produces into named problems that share a root anchor,
  with frozen pattern ids `god_file`, `hub`, `tangle`, `hot_mess`,
  `shotgun_pair`, `bus_risk`, `unstable_dependency`, and `measured`. Clustering
  creates no new measurement, rating, or verdict — it is grouping and naming
  over existing tables. Each finding is claimed by at most one card.
- **One `PROBLEMS` section replaces `FINDINGS`, `ARCHITECTURE`, and `HISTORY`
  in codebase mode.** `AREAS` and `WARNINGS` survive unchanged.
- **A one-screen budget.** Every default codebase view fits about one screen at
  every scope. A slot ladder trades card count against evidence depth so
  zooming changes which problems fill the budget, never how many lines print.
  `--top N` becomes N cards; `--all` shows every card including descriptive
  ones; a file scope shows its cards in full.
- **Dependency edges never print as rows in any human view, at any scope, at
  any detail level.** They survive only as aggregate card evidence such as
  `17 files import this`, and complete in the JSON relation tables. Unresolved
  and ambiguous relation rows print only when `--all` is supplied or the
  selected scope is a file; at every other scope the grouped `WARNINGS`
  sentence is their whole terminal presence.
- **A repository frame at sub-scope.** A package, directory, or file verdict
  gains an analysis-owned sentence stating how much of the repository's High
  debt lives in the selected scope.
- **A dependency specifier is always one line.** Unresolved targets are reduced
  to their first line with collapsed whitespace, so no raw source reaches
  output.
- **Diff mode keeps `FINDINGS`, `ARCHITECTURE`, and `HISTORY` this round.** It
  only stops printing edge rows. The resulting vocabulary split — cards in
  codebase mode, sections in diff mode — is deliberate for one cycle;
  card-ifying diff output is a follow-up change with its own comparison
  vocabulary, and doing it here would double the snapshot churn of an already
  large presentation change.
- **Trajectory tags are deferred.** Gate signals are deliberately
  time-invariant, so a card that claims a problem is getting worse cannot be
  proven by anything the gate ratchets. The problem evidence vocabulary is
  designed so a future trend variant is purely additive.

## Capabilities

### New Capabilities

- `problem-clustering`: the card model, frozen pattern ids, the claimed-once
  rule, deterministic integer-only detectors, the descriptive-card rule, the
  problem rank, and the build-once ordering contract.

### Modified Capabilities

- `terminal-output`: `PROBLEMS` replaces the three codebase sections; edges
  reach no human view at any scope or detail level, reversing the accepted
  path-drill retention; unresolved and ambiguous rows only under `--all` or at
  a file scope; the repository-share line in a sub-scope verdict block.
- `progressive-exploration`: the one-screen slot budget and its ladder; `--top`
  and `--all` in cards; problem rank replaces finding rank for codebase rows;
  cycle witnesses and history findings arrive as card evidence.
- `architecture-analysis`: the path-drill relations sentence is replaced; a new
  requirement makes an extracted dependency specifier single-line.
- `verdict-policy`: the analysis-owned repository-share fact for a sub-scope
  verdict.
- `report-schema-v4`: additive `problems` table, `verdict.share`, and the size
  finding position contract.
- `product-documentation`: the README documents cards, the budget, and the new
  meaning of `--top` and `--all`.
- `end-to-end-evidence`: black-box evidence for the budget at every scope, for
  card shapes, for the absence of edge rows, and for the single-line specifier.

### Referenced Without Change

- `hotspot-analysis`: the accepted finding rank and hotspot rule are reused by
  reference. Problem rank consumes the finding rank as one of its keys and this
  change does not restate or alter it.

## Impact

- Affects `smackdebt-analysis` (a new problem module, a size-finding identity
  type, the verdict share), `smackdebt-output` (the `PROBLEMS` section, the
  budget, deleted relationship and gated unresolved rows), and
  `smackdebt-languages` (single-line specifier text). `smackdebt-project`
  changes only where the report is finished; `smackdebt-discovery`,
  `smackdebt-git`, and the CLI are untouched apart from `--top`'s meaning.
- `schemas/report-v4.schema.json` gains the `problems` table and the verdict
  share. Version 4 stays the contract and gains members additively, following
  the precedent set when `earn-the-verdict` added coverage bytes and the
  verdict qualifier to v4 rather than opening a v5.
- Terminal snapshots churn twice by design: once when edge rows are deleted and
  once when sections become cards. The slices are ordered so each churn lands
  on an otherwise quiet baseline and every regenerated snapshot is reviewed per
  case. JSON snapshots churn once, in their own slice.
- The gate snapshot is built from finding tables and never from presentation, so
  no rendering change moves the ratchet baseline. The baseline still gains three
  reviewed entries across the change, each one real new source debt accepted with
  `gate --update` in the slice that wrote it: the `tests` container in
  `crates/output/src/output.rs` (one Watch container size, replacing one Watch
  cognitive), the new `crates/analysis/src/problem.rs` (one High file size), and
  the exhaustive evidence match in `crates/output/src/json.rs` (one Watch
  cyclomatic). `just gate` is green against the committed baseline at every
  slice.
- `ARCHITECTURE.md` is updated under its existing accepted
  `architecture-documentation` requirements, which already require it to
  describe the analysis and presentation boundary; no delta is needed there.
- Committed performance-baseline `report_digest` values go stale until a future
  `just release-baselines`, as they did for `earn-the-verdict`;
  `scripts/performance/check-baselines.py` validates committed records only, so
  `just performance-tests` stays green.
