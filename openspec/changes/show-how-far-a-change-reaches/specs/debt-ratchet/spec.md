## ADDED Requirements

### Requirement: Change-reach analytics stay outside the gate
The ratchet gate SHALL gain no signal from this change. Change-leakage findings,
change amplification, propagation reach, and core size SHALL NOT become gate
rows, SHALL NOT change an existing row, and SHALL NOT change the committed
baseline, so the accepted rule that ratcheted signals are time-invariant keeps
its force by exclusion rather than by omission: the first three derive from
history and would move with wall-clock time, and core size is a static fact this
change deliberately does not ratchet while its constants are under review.

#### Scenario: A leakage-bearing report is gated
- **WHEN** the gate snapshot of a tree carrying change-leakage findings, a core-size fact, and a reach fact is compared with the snapshot of the same tree analyzed without history
- **THEN** the two snapshots are identical row for row and byte for byte
