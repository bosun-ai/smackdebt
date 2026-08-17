## Why

The 2026-08-17 field evaluation on five real repositories — tokio,
scikit-learn, opencode, kwaak, and fluyt — confirmed that the verdicts and the
unit findings are real, and found that the two lines a reader trusts first are
the two lines that mislead.

- **The headline names test or benchmark code on 3 of 5 repositories.** tokio's
  `worst:` line named a `test_combination` unit, kwaak's named a benchmark
  `main.py`, and fluyt's named `run_swebench` while its genuine cognitive-190
  production `resolve` sat further down the FINDINGS list. The rank already
  carries `hot` and `role_class`, but both sit *after* signals at rating and
  total triggered signals. Those two counts almost never tie, so role and heat
  effectively never decide, and the accepted promise that "primary source
  precedes non-primary source at equal rating" is true only in a tie that does
  not occur in practice.
- **Diff `worse … added` cards carry no evidence.** An added unit prints the
  bare word `added` and nothing else, so the reader is told that new debt exists
  but never why the new unit counts as debt. The comparison already carries the
  added unit's after-side measurements, and the removed unit's before-side
  measurements; the renderer simply drops them.

Both defects damage trust in the first screen of output while the underlying
facts are correct. Both are ordering and formatting of facts analysis has
already produced.

## What Changes

- The source finding rank becomes rating, role class, hot state, signals at that
  rating, total triggered signals, cognitive complexity, cyclomatic complexity,
  logical lines, activity, path, and span. Production code outranks non-primary
  code at equal rating, and among production findings heat decides before signal
  counts. This reverses the accepted key order, whose hot-versus-signals
  scenarios are rewritten rather than dropped.
- Non-primary findings stay visible below primary findings, and a repository
  whose only rated findings are non-primary still names a worst offender. No
  worst-offender filter is added.
- Diff comparison cards for added and removed units keep their direction word
  and additionally show the present side's absolute measurements: the after side
  for `added`, the before side for `removed`, nonzero values only, direction
  word first. An added or removed unit whose measurements are all zero keeps the
  bare word.
- The README's rank sentence is corrected to the new sequence and stops calling
  logical lines `statements`.

## Capabilities

### Modified Capabilities

- `hotspot-analysis`: the finding rank key order and its tie scenarios.
- `product-documentation`: the README's documented rank sequence and its
  measurement vocabulary.
- `progressive-exploration`: added and removed diff cards state the present
  side's absolute measurements.

## Impact

This changes the order of FINDINGS rows, the `worst:` line, the `summary` worst
entries in JSON, and the terminal bytes of added and removed diff cards. It
affects `crates/analysis` rank policy, `crates/output` diff-card rendering, the
README and `ARCHITECTURE.md` rank paragraphs, and every terminal and JSON
snapshot whose finding order or diff cards move.

It does not change ratings, signals, measurements, verdict tiers, counts, exit
codes, the JSON schema shape, or which findings exist — only which of them is
read first, and what an added or removed card says. Change
`classify-production-architecture` is authored only after this change is
implemented and archived; `prepare-first-release` stays blocked.
