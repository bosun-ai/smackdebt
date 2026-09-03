# problem-clustering Specification

## Purpose
TBD - created by archiving change make-problems-legible. Update Purpose after archive.
## Requirements
### Requirement: Problems cluster existing findings without new measurement
Analysis SHALL group the findings a completed report already holds into problem
cards. A problem card SHALL carry a pattern id, a rating, one anchor, ordered
evidence, the identities of the findings it claimed, and its visibility. Every
value on a card SHALL be read from a table the report already
built: clustering SHALL NOT measure source, rate a unit, create a finding,
change a verdict, read a file, walk the filesystem, start a Git process, visit a
parser, or record an algorithm pass.

Cards SHALL be built once while the report is finished, after aggregation, and
SHALL be exposed as one table on the report. Clustering SHALL operate on
borrowed slices of the report's tables so each detector is testable without
composing a report.

An anchor SHALL be one of a file, a set of files, a package, or a package pair,
and SHALL be stored as an index into the table that owns that identity rather
than as a copied path.

#### Scenario: A card is produced
- **WHEN** a completed report is clustered
- **THEN** every card's rating, evidence value, and claimed finding comes from an existing table and no new measurement or rating exists

#### Scenario: Clustering is observed as work
- **WHEN** live work counters are compared immediately before and after the report is finished
- **THEN** clustering adds no inventory visit, read, Git process, parser visit, or algorithm pass

#### Scenario: A detector is tested alone
- **WHEN** a detector is exercised over hand-built slices
- **THEN** it produces its cards without a composed report

### Requirement: Pattern ids are a frozen machine vocabulary
Analysis SHALL own exactly these pattern ids: `god_file`, `hub`, `tangle`,
`hot_mess`, `shotgun_pair`, `bus_risk`, `unstable_dependency`, and `measured`.
The ids SHALL be the stable contract for machine consumers the way tier ids and
worst-offender reason ids are. No renderer SHALL invent, rename, or compose a
pattern id, and a renderer's human name for a pattern SHALL be a presentation
choice that never replaces the id in the machine report.

`measured` SHALL be the fallback pattern, so a verdict-affecting finding that no
named pattern claimed still reaches a card rather than disappearing.

#### Scenario: A machine consumer reads a pattern
- **WHEN** a card is serialized
- **THEN** its pattern is one of the eight frozen ids, taken from analysis rather than composed by the renderer

#### Scenario: A finding matches no named pattern
- **WHEN** a file carries a verdict-affecting finding that no named pattern claims
- **THEN** a `measured` card claims it

### Requirement: Every finding is claimed by exactly one card
Clustering SHALL claim findings in exactly this order: `tangle`, `god_file`,
`hub`, `hot_mess`, `shotgun_pair`, `bus_risk`, `unstable_dependency`,
`measured`. A finding already claimed by an earlier pattern SHALL NOT be claimed
again, and an index-integrity audit SHALL fail when one finding is claimed
twice.

A file-anchored pattern SHALL claim every retained finding of its file and every
size finding of its file, whether or not those findings affect the verdict.
Claiming SHALL therefore be blind to source role and to parse trust: advisory
trust and non-primary roles decide what a card is worth showing by default, never
whether its evidence survives clustering. Because every file-anchored pattern
claims that file's findings, at most one file-anchored card SHALL exist for a
file. This replaces the behavior where one file with several findings produced
one displayed row per finding.

Clustering SHALL leave no retained finding and no size finding unclaimed: with
`measured` as the fallback, every row of the source finding table, the size
finding table, the architecture finding table, the evolutionary finding table,
the knowledge-concentration finding table, and the stable-dependency finding
table SHALL be claimed by exactly one card. A coverage audit SHALL fail when a
retained finding reaches no card, for the same reason the index-integrity audit
fails when one reaches two.

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

#### Scenario: Every retained finding is accounted for
- **WHEN** a completed report is clustered
- **THEN** the number of distinct claimed findings equals the number of retained findings across every finding table, including size findings

### Requirement: File patterns are integer-only and package-relative
Every file pattern SHALL be decided from integer facts the report already owns,
and SHALL NOT produce or compare a floating-point value. File degree SHALL be
the dependency degree over edges that enter the architecture verdict graph, so
test, example, benchmark, module-ownership, and recovered relations contribute
nothing to a file's fan-in or fan-out.

- **`tangle`**: one card per architecture finding. Analysis already produces
  exactly one architecture finding per strongly connected component with one
  stable witness, so this pattern SHALL be a grouping of that existing finding
  and SHALL NOT create a card per member, per witness step, or per edge. The
  card's rating SHALL be the finding's rating, and its anchor SHALL be the
  finding's member files. A `tangle` SHALL claim its architecture finding only:
  the source findings of its member files SHALL remain available to the file
  patterns, because being inside a cycle and doing too much are two different
  problems.
- **`god_file`**: a file SHALL be a `god_file` when both of these hold:
  (a) it has at least 3 High findings, or at least 1 High finding and at least 6
  units rated Watch or High; **and** (b) it carries a size finding or has a
  fan-out of at least 10. Concentrated debt alone SHALL NOT be enough —
  conjunct (b) is what makes the pattern mean "does too much" rather than "has
  bugs" — and breadth alone SHALL NOT be enough either. Its rating SHALL be
  High. The second arm of conjunct (a) SHALL count units rated Watch or High
  and SHALL NOT count healthy units: a file's rated unit total is its length in
  units, so counting all of them names every long file that holds one bug,
  which single-file components produce by the hundred.
- **`hub`**: a file that no earlier file pattern claimed SHALL be a `hub` when
  its fan-in is at least 8 and either its package's median fan-in is zero or its
  fan-in is at least 4 times that median; the same rule SHALL apply to fan-out.
  The median SHALL be the nearest-rank median of the files of that file's own
  package, computed once per package, so it is an integer and does not depend on
  the selected scope. A `hub` card's rating SHALL be the highest rating among
  the findings it claims, whether or not those findings affect the verdict, and
  `healthy` only when it claims none. A generated, fixture, or recovered file
  that many primary files import SHALL therefore claim its own rated findings and
  carry their highest rating rather than being called healthy, and its visibility
  SHALL be `detail`.
- **`hot_mess`**: a file that no earlier file pattern claimed SHALL be a
  `hot_mess` when it is a hotspot and carries at least one High finding. Its
  rating SHALL be High, which agrees with the accepted `hot_and_complex`
  worst-offender reason.

The High findings the `god_file` and `hot_mess` rules count SHALL be findings
that affect the verdict, and the debt-carrying units the `god_file` rule counts
SHALL be the file's units rated Watch or High, which analysis already keeps at
zero for source that cannot move a verdict. A file whose debt cannot move a
verdict SHALL
therefore never be named a `god_file` or a `hot_mess`; it reaches a `hub` or a
`measured` card instead, which is what keeps those two names meaningful. Every
other decision a file pattern makes — what it claims, what rating it carries,
and what it states as evidence — SHALL be blind to role and trust.

The five thresholds — 3 High findings, 6 debt-carrying units, fan-out 10,
degree 8, and the 4-times median multiple — are proposed values under review and
SHALL be implemented as named integer constants so review can move them in one
place.

#### Scenario: A file has two High findings and nothing else
- **WHEN** the file has 2 High findings, no size finding, and fan-out 2
- **THEN** it is not a `god_file`, because neither arm of conjunct (a) holds

#### Scenario: A long file holds one bug
- **WHEN** a broad file has 1 High finding, 1 debt-carrying unit, and twenty healthy units beside it
- **THEN** it is not a `god_file`, because healthy units never satisfy the second arm of conjunct (a)

#### Scenario: A small file concentrates three High findings
- **WHEN** the file has 3 High findings, no size finding, and fan-out 2
- **THEN** it is not a `god_file`, because conjunct (b) fails even though conjunct (a) holds

#### Scenario: A file is both concentrated and broad
- **WHEN** the file has 3 High findings and a fan-out of 10
- **THEN** it is a `god_file` rated High

#### Scenario: A file is at the degree boundary
- **WHEN** one file has fan-in 7 and another has fan-in 8 in a package whose median fan-in is zero
- **THEN** only the file with fan-in 8 is a `hub`

#### Scenario: Degree sits exactly at the median multiple
- **WHEN** a file's fan-in is exactly 4 times its package's nonzero median fan-in and is at least 8
- **THEN** it is a `hub`

#### Scenario: Test imports do not create a hub
- **WHEN** a file's only incoming relations come from test, example, benchmark, module-ownership, or recovered source
- **THEN** its fan-in for pattern purposes is zero and it is not a `hub`

#### Scenario: The same file is clustered at two scopes
- **WHEN** the report is clustered once and read at repository, package, and directory scope
- **THEN** the file's pattern and rating are identical at every scope because the median is package-relative

#### Scenario: A cycle spans five files
- **WHEN** one architecture finding covers a strongly connected component of five files
- **THEN** exactly one `tangle` card exists for it, anchored on the member files, carrying the finding's existing witness as evidence

### Requirement: Table patterns keep their existing finding facts
`shotgun_pair`, `bus_risk`, and `unstable_dependency` SHALL each produce one card
per existing finding of their family — evolutionary coupling, knowledge
concentration, and stable-dependency violation respectively — carrying that
finding's rating and its existing operands as evidence, with no change to the
facts those findings already state.

`measured` SHALL produce one card per file that still holds at least one
unclaimed retained finding or at least one unclaimed size finding, and SHALL
claim all of them. Its head SHALL be the identity of that file's top finding
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

### Requirement: Card evidence is ordered head first
A file-anchored card SHALL state its evidence in exactly this order: the top
claimed source finding when it claims one, then the integer facts that made its
pattern fire, then each claimed size finding in table order, then the anchor's
touch count when the file is hot, then its remaining claimed source findings in
the accepted finding rank. A renderer under a budget shows a prefix of that
order, so the head names the problem, the next lines say why the pattern fired,
and the enumeration of every remaining finding comes last where `--all` reaches
it.

A `god_file` that fired on the size conjunct SHALL therefore state its size
finding among the facts that made it fire, directly after its rated unit total
and its fan-out, rather than after the findings it enumerates: the size finding
is the reason the card exists, not a trailing detail.

A `tangle` card SHALL state its architecture finding, then its member count,
then the touch count of its hottest member when one is hot. The witness comes
first because a renderer under a budget shows a prefix of the evidence: the
cycle is what the card is about, so it SHALL survive every rung that allows one
evidence line at all, while the member count is the fact the budget may drop.
A `shotgun_pair`, `bus_risk`, or `unstable_dependency` card SHALL state its one
finding.

#### Scenario: A god file is oversized
- **WHEN** a `god_file` fires because the file carries a size finding
- **THEN** its evidence reads top finding, rated units, size finding, and only then its remaining findings

#### Scenario: A card is rendered under a tight budget
- **WHEN** a rung allows one evidence line
- **THEN** the line shown is the card's head, which is its top claimed finding or, for a card claiming no source finding, its first other fact

### Requirement: A card's visibility decides where it is shown, never what it is
Every card SHALL carry a visibility of exactly one of two values, `default` or
`detail`. A card SHALL be `default` when it claims at least one finding that
affects the verdict, or when it anchors an architecture, coupling,
knowledge-concentration, or stable-dependency finding, all of which are rated by
the analyses that own them. Every other card SHALL be `detail`.

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

#### Scenario: A generated file is imported everywhere
- **WHEN** a generated file that satisfies the `hub` rule carries rated findings that cannot affect the verdict
- **THEN** its card claims them, carries their highest rating rather than `healthy`, and carries visibility `detail`

#### Scenario: A file's whole debt is advisory
- **WHEN** a recovered file carries only advisory findings
- **THEN** its `measured` card carries visibility `detail` and appears under `--all` in the accepted finding rank

#### Scenario: Visibility is filtered rather than ranked
- **WHEN** a `detail` card ranks between two `default` cards and default detail is rendered
- **THEN** the two `default` cards keep the order the rank gave them and no card is re-ranked

### Requirement: Problem rank is total and built once
Analysis SHALL order the problem table by rating descending, source priority,
claimed High count descending, hot before not hot, claimed finding count
descending, pattern in the frozen claiming order, the accepted finding rank of
the card's top claimed finding, anchor repository-relative path ascending, and
anchor start line ascending, in that order. Rating SHALL remain the strongest
priority.

Source priority SHALL put a card claiming any primary source finding first, a
card claiming no source finding second, and a card claiming source findings but
no primary source finding third. This keeps primary application debt ahead of
test, example, benchmark, fixture, and generated debt at the same rating while
giving architecture and history cards an explicit position. Source priority
SHALL NOT hide a card, change its visibility, rating, claims, counts, or verdict.
This specification SHALL NOT restate the keys of the finding rank, which
`hotspot-analysis` owns.

The order SHALL be total and data-stable, so serial and parallel runs produce
identical card order. The table SHALL be sorted once when the report is
finished, and no renderer SHALL sort, re-rank, or reorder it.

#### Scenario: Primary and benchmark cards share a rating
- **WHEN** a benchmark card claims more findings than a primary application card at the same rating
- **THEN** the primary application card appears first and the benchmark card remains visible lower in the ranked table

#### Scenario: Architecture has no source finding
- **WHEN** primary, architecture-only, and non-primary cards share a rating
- **THEN** they appear in that order before the remaining rank keys are considered

#### Scenario: Ratings differ
- **WHEN** a non-primary card is High and a primary application card is Watch
- **THEN** the High card appears first because rating remains the strongest priority

#### Scenario: Accepted terminal and machine views are rendered
- **WHEN** the public problem-pattern fixture is written in terminal and JSON form
- **THEN** both committed views carry the same primary, no-source, and non-primary card order

#### Scenario: Every rank key ties
- **WHEN** two cards tie on every key before the anchor
- **THEN** anchor path and anchor start line produce a stable order

#### Scenario: Serial and parallel runs cluster the same report
- **WHEN** the same selection is analyzed serially and in parallel
- **THEN** the card table is identical in content and order

### Requirement: A card belongs to a scope through its anchor
A card SHALL belong to a selected scope when its anchor lies within that scope: a
file anchor when the file is within the scope, a file-set anchor when any member
file is within the scope, and a package or package-pair anchor when the package
is within the scope or contains it. The report SHALL NOT store a per-scope card
list; a consumer SHALL filter the one ranked table by this rule, so ordering is
built once and scope selection adds no sort.

#### Scenario: A cycle crosses the selected directory boundary
- **WHEN** a `tangle` card's members lie partly inside and partly outside the selected directory
- **THEN** the card belongs to that directory's view once

#### Scenario: Two scopes are rendered from one report
- **WHEN** a package and one of its directories are rendered from the same completed report
- **THEN** each view filters the same ranked table and no second ordering is computed

