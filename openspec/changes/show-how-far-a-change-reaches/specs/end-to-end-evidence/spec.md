## MODIFIED Requirements

### Requirement: Human views carry no dependency edge rows
Every committed human view, at every scope and every detail level, SHALL be
proven free of dependency-edge rows: no row heading two repository file paths
with an arrow or with ` owns `, no line stating the single-reference import fact
`· 1 import`, and no arrow joining two repository file paths outside a cycle
witness or a co-change finding. The check SHALL be an invariant over every
committed terminal snapshot rather than an assertion on selected cases, so a
future view cannot reintroduce the rows quietly.

The check SHALL be written against file paths, because two package identities
joined by an arrow are not an edge row: an `unstable_dependency` card head keeps
the accepted `<source> → <target>` package wording, and its reference-count
evidence is never the single-reference form, since a stable-dependency finding
requires at least two references.

The check SHALL admit exactly one further shape: the two file paths a co-change
finding names, as a `hidden_coupling` card head or as a leakage evidence line. A
finding's subject is not a relation row, and the invariant SHALL prove the
distinction rather than blur it — a card carrying those two paths SHALL still be
proven to carry no reference count, no relation kind, no resolution outcome, and
no ownership wording. The documentation of the invariant SHALL state the
carve-out where the check is written, so a later reader is not left to infer it
from a passing test.

Evidence SHALL prove that unresolved and ambiguous rows appear at a file scope
and under `--all`, and that at every other scope the grouped warning sentence is
their whole terminal presence.

#### Scenario: Every snapshot is scanned
- **WHEN** the committed terminal snapshots are scanned as a set
- **THEN** none heads two repository file paths with an arrow or with ` owns `, none states `· 1 import`, and the only two-file-path shapes are cycle witnesses and co-change findings

#### Scenario: A co-change card is scanned
- **WHEN** a snapshot containing a `hidden_coupling` card and a leakage evidence line is scanned
- **THEN** the invariant passes and the same card is proven to carry no reference count, relation kind, resolution outcome, or ownership wording

#### Scenario: An unfollowed import is inspected
- **WHEN** the same fixture is rendered at a directory scope, at a file scope, and with `--all`
- **THEN** only the file scope and the `--all` run print per-import rows and all three print the grouped sentence

## ADDED Requirements

### Requirement: Change leakage has exact black-box evidence
A generated repository SHALL provide hand-calculated facts for both leakage
kinds and for every case that must produce nothing: an importer that follows its
interface across three directories, a provably unlinked cross-directory pair
that the package stage separates, a pair inside one directory, and a test file
that changes with the primary file it exercises. Exact acceptance SHALL prove
the presence of the two findings and the absence of the other two, in the
terminal and in the machine report from the same invocation.

Threshold-boundary evidence — one commit either side of the support floor, one
directory either side of the distance floor, and the similarity bar at each
distance step — SHALL be pure policy tests beside the rules rather than
generated repository fixtures, exactly as the accepted card boundary evidence
is. A repository built to hold a threshold states that threshold twice and moves
whenever review moves the constant.

Evidence SHALL prove the strict launch: over that fixture the default view gains
at most one card, the one-screen budget still holds at every scope, and a
retained pair below every detector floor appears in the machine report while no
terminal view — default, at that file's own scope, or `--all` — states it.

Evidence SHALL prove that the leakage tables and the cards they produce are
identical under one worker and automatic parallelism.

#### Scenario: The leakage fixture runs
- **WHEN** the real CLI analyzes the leakage fixture
- **THEN** one `leaky_interface` finding and one `hidden_coupling` finding exist with their hand-calculated operands and distances, and no finding names the same-directory pair or the test file

#### Scenario: The budget still holds
- **WHEN** the leakage fixture is rendered by default at repository, package, and directory scope
- **THEN** each view fits the one-screen budget and gains at most one new card against the same fixture rendered without history

#### Scenario: A below-floor pair is requested
- **WHEN** the fixture's below-floor pair is looked for by default, at its file scope, and under `--all`
- **THEN** no view states it and the machine report retains its row

### Requirement: Propagation reach and core size have exact evidence
Generated repositories SHALL provide hand-calculated facts for propagation reach
and core size, including their absence cases: a layered multi-package repository
whose root verdict states a reach sentence, a package large enough to state a
file reach sentence, a single-package repository below the file floor that
states no reach at all, a single-package repository above it whose root states
that package's file reach, and a repository whose largest file cycle is too
small a share of the codebase to be stated. Exact acceptance SHALL prove the sentence bytes in the terminal and the
same operands in the machine report from one invocation, and SHALL prove that
the verdict tier, its counts, and the worst offender are identical whether or
not each fact exists.

Evidence SHALL prove that the head sentences survive the required widths: every
new sentence SHALL be rendered at 50 columns and lose no fact, and no operand
SHALL be clipped.

Evidence SHALL prove index integrity for the four new tables through the machine
report — every file, package, and coupling index resolving inside the table it
names — and that no floating-point number or boolean appears in any of them.

#### Scenario: A layered repository is analyzed
- **WHEN** the real CLI analyzes the propagation fixture at its root and at one package
- **THEN** the root states the package reach sentence and the package states the file reach sentence with their hand-calculated integers

#### Scenario: A single-package repository is analyzed
- **WHEN** the fixture declaring one package below the file floor is analyzed
- **THEN** no reach sentence appears in the terminal and no reach member appears in the machine report

#### Scenario: A single-package repository is large enough to state its reach
- **WHEN** the fixture declaring one package above the file floor is analyzed at its root
- **THEN** the root states that package's file reach sentence with its hand-calculated integers and the machine report carries the same operands

#### Scenario: A small core is analyzed
- **WHEN** the fixture whose largest file cycle holds three of two hundred files is analyzed
- **THEN** no core size sentence appears and the cycle's own findings are unchanged

#### Scenario: A new sentence is read at fifty columns
- **WHEN** a verdict carrying all three new sentences is rendered at 50 columns
- **THEN** each sentence is stated in full, stacking rather than clipping, and every operand is present

### Requirement: Change amplification has exact evidence
A generated repository SHALL provide hand-calculated facts for change
amplification: an exact nearest-rank median at a repository, package, and
directory scope; a commit counted once per directory rather than once per file;
a scope with too few commits stating nothing; and a file scope stating nothing.
Exact acceptance SHALL prove the sentence bytes and the serialized operands from
one invocation, and SHALL prove the fact never moves the tier, its counts, or the
worst offender.

A generated repository SHALL additionally prove the bulk-commit guard end to
end: a commit touching thirty files that enter the change graph SHALL yield a
bulk commit count of one, no pair from that commit, and unchanged churn, touch,
package coupling, concentration, and amplification values.

#### Scenario: The amplification fixture runs
- **WHEN** the real CLI analyzes the amplification fixture at three scopes
- **THEN** each states its hand-calculated median and commit count, and the file scope states nothing

#### Scenario: The bulk commit fixture runs
- **WHEN** the real CLI analyzes the bulk-commit fixture
- **THEN** history coverage states one bulk commit, no pair exists from that commit, and every other history value equals the value from the same fixture without the bulk commit

### Requirement: Retired terminal wording returns when a card revives it
The README vocabulary audit SHALL stop banning the phrase `change together
without a dependency` and SHALL require it instead. The ban was correct while
the phrase belonged to a deleted coupling section: it existed to prove the
section had not crept back into the documented output. The phrase is now the
human name of the frozen pattern id `hidden_coupling`, so the documentation must
carry it and an audit that forbids it would forbid documenting a shipped
feature.

The audit's other banned phrases SHALL be unchanged, and the change that lifts
this one SHALL record the reason where the assertion is written, so a later
reader can tell a deliberate revival from an accidental regression.

#### Scenario: The README vocabulary is audited
- **WHEN** the documentation audit runs after the leakage cards exist
- **THEN** it requires `change together without a dependency`, keeps every other banned phrase banned, and the deleted assertion's reason is recorded beside the audit
