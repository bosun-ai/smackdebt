## Why

The final whole-branch review of the "trust the headline" wave found one stale
document. The accepted requirement `Codebase child order is severity-led` in
`progressive-exploration` still enumerates the pre-branch finding rank —
"rating, count of signals at that rating, total triggered signals, … recent
activity" — with no role class and no hot state. Change
`put-production-debt-first` moved role class and hot state ahead of both signal
counts in `hotspot-analysis`, so the enumeration now contradicts the accepted
rank it copies, and its scenario ("it appears first even when the other has
greater recent activity") states a tie-break that hot state decides first.

The enumeration also does not describe the code it governs. Terminal codebase
child area rows sort by High descending, Watch descending, then name ascending
(`codebase_child_order`); only the finding rows use the full rank
(`finding_order` over `FindingRank`). One requirement was describing two
different orders and matched neither.

The fix is documentation only: the implemented behavior is already the accepted
behavior in `hotspot-analysis`. Restating the rank keys in a second capability
is what allowed the drift, so the rewritten requirement names the child-row keys
it actually owns and defers to the finding rank by reference instead of copying
it.

## What Changes

- `Codebase child order is severity-led` states the child area row order it
  governs: High descending, Watch descending, then repository-relative name
  ascending.
- The same requirement defers finding order to the finding rank owned by
  `hotspot-analysis` by reference, without re-enumerating its keys, so the two
  documents cannot drift apart again.
- Its scenarios are rewritten to hot-aware, role-aware outcomes that agree with
  the accepted rank.

## Capabilities

### Modified Capabilities

- `progressive-exploration`: the codebase child order requirement and its
  scenarios.

## Impact

Documentation only. No crate, test, snapshot, schema, or exit code changes; the
implemented ordering is unchanged and already matches the rewritten text. This
change is authored, ticked, and archived in one step because it carries zero
implementation. `prepare-first-release` stays blocked.
