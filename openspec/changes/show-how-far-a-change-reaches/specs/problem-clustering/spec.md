## MODIFIED Requirements

### Requirement: Pattern ids are a frozen machine vocabulary
Analysis SHALL own exactly these pattern ids: `god_file`, `hub`, `tangle`,
`hot_mess`, `shotgun_pair`, `bus_risk`, `unstable_dependency`, `measured`,
`leaky_interface`, and `hidden_coupling`.
The ids SHALL be the stable contract for machine consumers the way tier ids and
worst-offender reason ids are. No renderer SHALL invent, rename, or compose a
pattern id, and a renderer's human name for a pattern SHALL be a presentation
choice that never replaces the id in the machine report.

This replaces the eight-id vocabulary with a ten-id one. The two new ids are
appended after `measured` rather than inserted among the existing ids, because
declaration order is claiming order: appending at the tail leaves every existing
claim, every existing card, and every existing rank position byte-identical, and
makes the growth of the vocabulary provable rather than argued.

`measured` SHALL remain the fallback pattern for findings that name a file, so a
verdict-affecting finding that no named pattern claimed still reaches a card
rather than disappearing. It SHALL NOT be the fallback for a change-leakage
finding, which has its own two patterns behind it.

#### Scenario: A machine consumer reads a pattern
- **WHEN** a card is serialized
- **THEN** its pattern is one of the ten frozen ids, taken from analysis rather than composed by the renderer

#### Scenario: A finding matches no named pattern
- **WHEN** a file carries a verdict-affecting finding that no named pattern claims
- **THEN** a `measured` card claims it

#### Scenario: The vocabulary grows
- **WHEN** cards built before and after the two ids were added are compared over the same report
- **THEN** every card that existed before keeps its pattern, its claims, and its rank position

### Requirement: Every finding is claimed by exactly one card
Clustering SHALL claim findings in exactly this order: `tangle`, `god_file`,
`hub`, `hot_mess`, `shotgun_pair`, `bus_risk`, `unstable_dependency`,
`measured`, `leaky_interface`, `hidden_coupling`. A finding already claimed by an
earlier pattern SHALL NOT be claimed again, and an index-integrity audit SHALL
fail when one finding is claimed twice.

A file-anchored pattern SHALL claim every retained finding of its file and every
size finding of its file, whether or not those findings affect the verdict.
Claiming SHALL therefore be blind to source role and to parse trust: advisory
trust and non-primary roles decide what a card is worth showing by default, never
whether its evidence survives clustering. Because every file-anchored pattern
claims that file's findings, at most one file-anchored card SHALL exist for a
file. This replaces the behavior where one file with several findings produced
one displayed row per finding.

A change-leakage finding SHALL belong, for claiming, to exactly one file: a
`leaky_interface` finding to the interface whose importers follow it, and a
`hidden_coupling` finding to the lower-indexed of its two files. A file-anchored
card therefore claims the leakage findings that belong to its file, which is what
makes the leakage numbers evidence on an existing card rather than a second card
about the same file.

Clustering SHALL leave no retained finding and no size finding unclaimed: with
`measured` as the fallback, every row of the source finding table, the size
finding table, the architecture finding table, the evolutionary finding table,
the knowledge-concentration finding table, the stable-dependency finding table,
and the change-leakage finding table SHALL be claimed by exactly one card. A
coverage audit SHALL fail when a retained finding reaches no card, for the same
reason the index-integrity audit fails when one reaches two.

#### Scenario: One file carries three High findings
- **WHEN** the file is clustered
- **THEN** exactly one file-anchored card exists for it and it claims all three findings

#### Scenario: A file is both oversized and widely imported
- **WHEN** the file satisfies both the `god_file` and the `hub` rule
- **THEN** the `god_file` card claims it because `god_file` precedes `hub` in claiming order

#### Scenario: A finding is claimed twice
- **WHEN** clustering would attach one finding to two cards
- **THEN** the index-integrity audit fails

#### Scenario: A file's only debt is advisory
- **WHEN** a recovered file carries rated findings that cannot affect the verdict
- **THEN** a card claims all of them rather than leaving them unclaimed, and the coverage audit passes

#### Scenario: A leaky interface also does too much
- **WHEN** a file that satisfies the `god_file` rule is also the interface of a `leaky_interface` finding
- **THEN** the `god_file` card claims that finding and no second card names the file

#### Scenario: Every retained finding is accounted for
- **WHEN** a completed report is clustered
- **THEN** the number of distinct claimed findings equals the number of retained findings across every finding table, including size findings and change-leakage findings

### Requirement: Table patterns keep their existing finding facts
`shotgun_pair`, `bus_risk`, and `unstable_dependency` SHALL each produce one card
per existing finding of their family — evolutionary coupling, knowledge
concentration, and stable-dependency violation respectively — carrying that
finding's rating and its existing operands as evidence, with no change to the
facts those findings already state.

`measured` SHALL produce one card per file that still holds at least one
unclaimed source finding or at least one unclaimed size finding, and SHALL
claim all of them, together with any change-leakage finding that belongs to that
file. Its trigger SHALL be those two families only: an unclaimed change-leakage
finding SHALL NOT bring a `measured` card into existence, because the two
leakage patterns behind `measured` in claiming order exist to name exactly that
case, and a fallback that fired first would take the name away from them. Its
head SHALL be the identity of that file's top finding
under the accepted finding rank, so a `measured` card states exactly what a
displayed finding row states today, and SHALL be that file's first size finding
when it claims no source finding at all. Its rating SHALL be the highest rating
among the findings it claims.

A file whose only unclaimed evidence is a size finding SHALL therefore reach a
`measured` card carrying that size finding, so the size row today's `--all` view
prints keeps a home. A file whose only unclaimed findings are advisory or
non-primary SHALL likewise reach a `measured` card, so recovered and non-primary
debt keeps the ranking it has today inside `--all` instead of disappearing.

#### Scenario: A coupling finding is clustered
- **WHEN** an unexplained coupling finding exists for a package pair
- **THEN** one `shotgun_pair` card anchored on that pair carries the finding's shared commits, union commits, similarity operands, and dependency state unchanged

#### Scenario: A concentration finding is clustered
- **WHEN** a knowledge-concentration finding exists
- **THEN** one `bus_risk` card carries its contributor count, numerator, and denominator and no contributor identity

#### Scenario: A file has one ordinary Watch finding
- **WHEN** no named pattern claims the file
- **THEN** one `measured` card names that file's top ranked finding and claims its remaining findings

#### Scenario: A file's only unclaimed evidence is its size
- **WHEN** a file carries a size finding and no unclaimed source finding
- **THEN** one `measured` card claims that size finding, heads on it, and takes its rating

#### Scenario: A file's only unclaimed evidence is a leakage finding
- **WHEN** a file holds no unclaimed source finding and no unclaimed size finding, and one `leaky_interface` finding names it
- **THEN** no `measured` card exists for that file and the finding reaches a `leaky_interface` card instead

#### Scenario: A measured card also holds a leakage finding
- **WHEN** a file reaches a `measured` card through an unclaimed source finding and a `hidden_coupling` finding belongs to it
- **THEN** that card claims both and states the leakage finding as evidence

### Requirement: Card evidence is ordered head first
A file-anchored card SHALL state its evidence in exactly this order: the top
claimed source finding when it claims one, then the integer facts that made its
pattern fire, then the anchor's exact propagation reach when the pattern is
`hub` and a reach value greater than zero exists, then each claimed
change-leakage finding in
finding-table order, then each claimed size finding in table order, then the
anchor's touch count when the file is hot, then its remaining claimed source
findings in the accepted finding rank. A renderer under a budget shows a prefix
of that order, so the head names the problem, the next lines say why the pattern
fired, and the enumeration of every remaining finding comes last where `--all`
reaches it.

A `hub` fires on either half of its degree, so a file that imports many others
and is imported by none is a candidate whose reach is zero. `a change here
reaches 0 files` states nothing, and an immaterial fact is absent rather than
printed, so the candidate table keeps the row and the card states no reach line.

A `god_file` that fired on the size conjunct SHALL therefore state its size
finding among the facts that made it fire — after its rated unit total, its
fan-out, and any change-leakage finding its file carries — rather than after the
findings it enumerates: the size finding is the reason the card exists, not a
trailing detail. Leakage precedes it because the one order above is the whole
rule; the size finding's guarantee is that the enumerated findings never come
between it and the facts that made the pattern fire.

A `tangle` card SHALL state its architecture finding, then its member count,
then its exact propagation reach when one exists, then the touch count of its
hottest member when one is hot. The witness comes
first because a renderer under a budget shows a prefix of the evidence: the
cycle is what the card is about, so it SHALL survive every rung that allows one
evidence line at all, while the member count is the fact the budget may drop.
Reach follows the member count because how far the cycle spreads is a fact about
the cycle, and it precedes heat for the same reason.
A `shotgun_pair`, `bus_risk`, or `unstable_dependency` card SHALL state its one
finding.

A `leaky_interface` card SHALL state how many importers follow the interface,
then one line per claimed finding in finding-table order, so the count survives
the tightest rung and the followers are the detail a wider rung buys. A
`hidden_coupling` card SHALL state its one finding.

#### Scenario: A god file is oversized
- **WHEN** a `god_file` fires because the file carries a size finding
- **THEN** its evidence reads top finding, rated units, size finding, and only then its remaining findings

#### Scenario: A hub spreads
- **WHEN** a `hub` card's file is inside the reach candidate set
- **THEN** its evidence states the importer count that made it fire and then its exact reach, before any claimed size finding

#### Scenario: A hub only imports
- **WHEN** a `hub` fired on its fan-out and nothing depends on its file
- **THEN** its reach is zero, its card states no reach line, and the candidate table still holds the row

#### Scenario: A card is rendered under a tight budget
- **WHEN** a rung allows one evidence line
- **THEN** the line shown is the card's head, which is its top claimed finding or, for a card claiming no source finding, its first other fact

#### Scenario: A leaky interface has three followers
- **WHEN** a standalone `leaky_interface` card claims three findings
- **THEN** its first evidence line states the follower count and its next lines state one follower each

### Requirement: A card's visibility decides where it is shown, never what it is
Every card SHALL carry a visibility of exactly one of two values, `default` or
`detail`. A card SHALL be `default` when it claims at least one finding that
affects the verdict, when it claims at least one change-leakage finding, or when
it anchors an architecture, coupling,
knowledge-concentration, or stable-dependency finding, all of which are rated by
the analyses that own them. Every other card SHALL be `detail`.

Claiming a change-leakage finding is enough because such a finding is a rated
statement that survived every guard, floor, and proof its capability demands.
Without this arm a widely imported file with no debt of its own — the exact file
a leaky interface tends to be — would carry its leakage evidence on a `detail`
card and vanish from the default view, which would make the finding unreachable
in the view it was built for.

A `detail` card SHALL be shown only when `--all` is supplied or when the card's
anchor is the selected scope itself, and SHALL remain present in the machine
report at every detail level. This replaces the earlier boolean that called one
card shape descriptive: the healthy `hub` that claims nothing is now the
`detail` card that happens to be `healthy`, and it is no longer the only card
kept out of default detail. A file whose whole debt is advisory, non-primary, or
a size measurement also carries a `detail` card, which is exactly the content
today's default view already withholds and today's `--all` view already shows.

Visibility SHALL NOT be a key of the problem rank and SHALL NOT change a rating,
a count, or a verdict. A view SHALL apply it by filtering the ranked table, so
removing a `detail` card never changes the relative order of the cards that
remain.

Every card SHALL be rated by the findings it claims, so a card is `healthy` only
when it claims nothing, which only `hub` can do.

#### Scenario: A healthy utility is imported everywhere
- **WHEN** a file with 40 importers carries no finding and no size finding
- **THEN** its `hub` card claims nothing, carries rating `healthy` and visibility `detail`, and is absent from default terminal detail

#### Scenario: A widely imported file also carries debt
- **WHEN** a file that satisfies the `hub` rule carries a High finding that affects the verdict
- **THEN** its card is rated High, carries visibility `default`, and appears in default detail

#### Scenario: A healthy hub leaks
- **WHEN** a file with 40 importers carries no finding of its own and one `leaky_interface` finding names it
- **THEN** its `hub` card claims that finding, carries the finding's Watch rating, carries visibility `default`, and states the leakage evidence in the default view

#### Scenario: A generated file is imported everywhere
- **WHEN** a generated file that satisfies the `hub` rule carries rated findings that cannot affect the verdict
- **THEN** its card claims them, carries their highest rating rather than `healthy`, and carries visibility `detail`

#### Scenario: A file's whole debt is advisory
- **WHEN** a recovered file carries only advisory findings
- **THEN** its `measured` card carries visibility `detail` and appears under `--all` in the accepted finding rank

#### Scenario: Visibility is filtered rather than ranked
- **WHEN** a `detail` card ranks between two `default` cards and default detail is rendered
- **THEN** the two `default` cards keep the order the rank gave them and no card is re-ranked

## ADDED Requirements

### Requirement: Leakage patterns produce a card only when nothing else names the subject
The two leakage patterns SHALL run after every other pattern and SHALL card only
what is left, so a leakage number reaches a reader as one more line on the card
that already names its subject wherever such a card exists, and as a card of its
own only where none does. A second card about a file the report already names
would state the same subject twice inside a budget built to name each problem
once.

- **`leaky_interface`** SHALL produce one card per interface file that still
  holds at least one unclaimed `leaky_interface` finding, and SHALL claim every
  such finding of that file. Its anchor SHALL be that file.
- **`hidden_coupling`** SHALL produce one card per unclaimed `hidden_coupling`
  finding. Its anchor SHALL be the two files of the pair, which is the existing
  file-set anchor kind, so no new anchor kind is introduced.

A `hidden_coupling` finding SHALL be claimed by the card of its lower-indexed
file when that file already carries a file-anchored card, and SHALL otherwise
reach a card of its own. The lower-indexed file is a fixed, data-stable choice:
letting either endpoint claim the finding would make the pair's evidence land
under whichever file happened to carry debt, so the same report would state the
same pair in two different places depending on unrelated facts.

A file whose only unclaimed evidence is a leakage finding therefore reaches that
finding's own pattern rather than the fallback, because `measured` is triggered
by unclaimed source and size findings alone under the rule that owns it, which
this specification does not restate. A `measured` card that exists for other
reasons still claims its file's leakage findings, the way every file-anchored
card does.

Every card of both patterns SHALL take the Watch rating of the findings it
claims, and both patterns SHALL create no measurement, no rating, and no finding
of their own, exactly as every other pattern.

#### Scenario: The interface already carries a card
- **WHEN** a file that is a `hot_mess` is also the interface of two `leaky_interface` findings
- **THEN** the `hot_mess` card claims both findings, states them as evidence, and no `leaky_interface` card exists

#### Scenario: The interface carries nothing else
- **WHEN** an interface file holds no source finding, no size finding, and two `leaky_interface` findings
- **THEN** one `leaky_interface` card anchored on that file claims both

#### Scenario: A hidden pair has no other card
- **WHEN** neither file of a `hidden_coupling` finding carries a file-anchored card
- **THEN** one `hidden_coupling` card anchored on both files claims the finding

#### Scenario: The lower-indexed file carries a card
- **WHEN** the lower-indexed file of a `hidden_coupling` finding carries a `god_file` card
- **THEN** that card claims the finding and no `hidden_coupling` card exists for the pair

#### Scenario: Only the higher-indexed file carries a card
- **WHEN** the higher-indexed file of a `hidden_coupling` finding carries a card and the lower-indexed file does not
- **THEN** one `hidden_coupling` card claims the finding and the other file's card is unchanged
