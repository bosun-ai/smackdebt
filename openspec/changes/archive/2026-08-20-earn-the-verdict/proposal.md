## Why

The 2026-08-20 field evaluation — smackdebt on itself, 59 repositories under
`~/projects`, and landscape research — showed the verdict, the headline
feature, cannot yet be trusted in the field:

- Gitignore handling is a hand-rolled approximation. Anchored patterns match at
  any depth, negations do not work, nested `.gitignore`, `.git/info/exclude`,
  and the global gitignore are not read.
- Nested worktree checkouts inside an analyzed repository triple-counted
  findings on a real repository.
- About 95% of runtime streams full git history to fill a 90-day window, and an
  empty history window is silently undisclosed.
- Tier policy is pure density: 1 High in 5 checked units reads `lost` while 545
  High in 102k reads `worn`.
- Grouped Rust imports (`use crate::{A, b, c};`) collapse to one bare `crate`
  reference, dropping item-level dependency fidelity repository-wide.
- Coupling rows claim `no code dependency` for pairs linked through a real
  transitive dependency path.
- The repository has no CI and no LICENSE, and a dead architecture check passes
  green unconditionally.

This change fixes trust in discovery and history, sharpens the verdict, adds
the promised policy gate as a self-dogfooded ratchet, and cleans up the
apparatus.

## What Changes

- Discovery is rewritten on the `ignore` crate with full git semantics: root
  and nested `.gitignore`, `.git/info/exclude`, and the global gitignore,
  including anchoring and `!` negation. Configuration excludes become gitignore
  syntax anchored at the analyzed root. Dependency directories stay excluded
  regardless of negation.
- Nested git checkouts — a subdirectory with a `.git` directory or `.git`
  file — are always excluded and disclosed as a `nested_repository` diagnostic
  with a default-output sentence.
- The history window is pushed into the git process and filters on when a
  change landed; an empty window is disclosed with a warning sentence.
- Grouped Rust `use` lists emit one reference per imported item.
- Coupling wording distinguishes `no direct dependency` with `linked via
  <package>` from a genuinely unlinked `no code dependency`.
- Unfollowed-import warnings break down into their causes.
- The codebase tier gains a small-scope evidence cap and absolute debt-volume
  floors, and the verdict carries a qualifier when a large share of source is
  unsupported.
- `--top N` becomes a middle level of terminal detail for findings.
- A new `smackdebt gate` command ratchets debt against a committed baseline
  with exit code 3, `--update`, and JSON output; smackdebt gates itself in
  `just check`.
- Ops: MIT LICENSE, GitHub Actions CI running the complete check, the dead
  evidence-map script retired, JSON snapshots reviewable through a `diff=json`
  textconv, and a hermetic acceptance child environment.

## Capabilities

### New Capabilities

- `source-discovery`: Git-faithful ignore semantics, always-excluded dependency
  directories, nested-checkout exclusion, configuration excludes as gitignore
  syntax, deterministic order.
- `debt-ratchet`: The committed baseline, ratcheted signals, regression and
  improvement rules, `--update`, exit code 3, gate JSON, and self-gating.

### Modified Capabilities

- `verdict-policy`: Small-scope cap and volume floors in tier mapping; an
  unsupported-coverage qualifier.
- `evolutionary-analysis`: The window filter moves into the streamed history
  process with landed-date semantics, coverage fields describe the windowed
  set, an empty window is a visible limitation, and coupling explanation gains
  a transitive link classification.
- `terminal-output`: Empty-window sentence, unfollowed-import breakdown,
  nested-repository sentence, `--top` and gate command text.
- `architecture-analysis`: One reference per imported item in grouped imports.
- `language-analysis`: Grouped `use` lists resolve each imported item.
- `progressive-exploration`: `--top` as a middle detail level; qualifier row
  placement under the verdict block.
- `report-schema-v4`: `nested_repository` diagnostic kind, coverage byte
  fields, and the verdict qualifier.
- `product-documentation`: README documents the gate and exit 3, discovery
  rules, `--top`, the coupling wording, and the license.
- `end-to-end-evidence`: Hermetic child environment; gate evidence.
- `release-readiness`: Every commit runs the complete gate in CI.
- `source-signal-quality`: Generated names remain non-ignores under the new
  discovery engine.

## Impact

- Affects `smackdebt-discovery` (rewritten walk), `smackdebt-git` (windowed
  streaming), `smackdebt-languages` (grouped-use extraction),
  `smackdebt-analysis` (tier terms, qualifier, coupling link, gate policy),
  `smackdebt-project` (window threading), `smackdebt-output` (new sentences,
  `--top`, gate rendering), and the CLI (arguments, `gate` subcommand, exit
  code 3).
- `schemas/report-v4.schema.json` gains the `nested_repository` diagnostic
  kind, coverage byte fields, and the verdict qualifier; a new
  `schemas/gate-v1.schema.json` is added.
- Terminal and JSON snapshots churn where grouped-use resolution, tier terms,
  and coverage fields move them; every regenerated snapshot is reviewed per
  case. API snapshots change for discovery, git, project, and analysis where
  signatures move.
- Committed performance-baseline `report_digest` values go stale until a future
  `just release-baselines`; `scripts/performance/check-baselines.py` validates
  committed records only, so `just performance-tests` stays green.
- This wave is not part of `prepare-first-release`'s dependency order; that
  change stays coherent and remains blocked on its own terms.
