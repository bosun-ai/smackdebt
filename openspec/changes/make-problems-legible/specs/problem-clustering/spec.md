## ADDED Requirements

### Requirement: Problems cluster existing findings without new measurement
Analysis SHALL group the findings a completed report already holds into problem
cards. A problem card SHALL carry a pattern id, a rating, one anchor, ordered
evidence, the identities of the findings it claimed, and whether it is
descriptive. Every value on a card SHALL be read from a table the report already
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

### Requirement: Every finding is claimed by at most one card
Clustering SHALL claim findings in exactly this order: `tangle`, `god_file`,
`hub`, `hot_mess`, `shotgun_pair`, `bus_risk`, `unstable_dependency`,
`measured`. A finding already claimed by an earlier pattern SHALL NOT be claimed
again, and an index-integrity audit SHALL fail when one finding is claimed
twice.

Because every file-anchored pattern claims that file's findings, at most one
file-anchored card SHALL exist for a file. This replaces the behavior where one
file with several findings produced one displayed row per finding.

#### Scenario: One file carries three High findings
- **WHEN** the file is clustered
- **THEN** exactly one file-anchored card exists for it and it claims all three findings

#### Scenario: A file is both oversized and widely imported
- **WHEN** the file satisfies both the `god_file` and the `hub` rule
- **THEN** the `god_file` card claims it because `god_file` precedes `hub` in claiming order

#### Scenario: A finding is claimed twice
- **WHEN** clustering would attach one finding to two cards
- **THEN** the index-integrity audit fails

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
  (a) it has at least 3 High findings, or at least 1 High finding and at least 8
  rated units; **and** (b) it carries a size finding or has a fan-out of at
  least 10. Concentrated debt alone SHALL NOT be enough — conjunct (b) is what
  makes the pattern mean "does too much" rather than "has bugs" — and breadth
  alone SHALL NOT be enough either. Its rating SHALL be High.
- **`hub`**: a file that no earlier file pattern claimed SHALL be a `hub` when
  its fan-in is at least 8 and either its package's median fan-in is zero or its
  fan-in is at least 4 times that median; the same rule SHALL apply to fan-out.
  The median SHALL be the nearest-rank median of the files of that file's own
  package, computed once per package, so it is an integer and does not depend on
  the selected scope. A `hub` card's rating SHALL be the highest rating among
  the findings it claims, and `healthy` when it claims none, which is the
  descriptive case defined below.
- **`hot_mess`**: a file that no earlier file pattern claimed SHALL be a
  `hot_mess` when it is a hotspot and carries at least one High finding. Its
  rating SHALL be High, which agrees with the accepted `hot_and_complex`
  worst-offender reason.

The five thresholds — 3 High findings, 8 rated units, fan-out 10, degree 8, and
the 4-times median multiple — are proposed values under review and SHALL be
implemented as named integer constants so review can move them in one place.

#### Scenario: A file has two High findings and nothing else
- **WHEN** the file has 2 High findings, no size finding, and fan-out 2
- **THEN** it is not a `god_file`, because neither arm of conjunct (a) holds

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
unclaimed verdict-affecting finding. Its head SHALL be the identity of that
file's top finding under the accepted finding rank, so a `measured` card states
exactly what a displayed finding row states today. Its rating SHALL be the
highest rating among the findings it claims.

#### Scenario: A coupling finding is clustered
- **WHEN** an unexplained coupling finding exists for a package pair
- **THEN** one `shotgun_pair` card anchored on that pair carries the finding's shared commits, union commits, similarity operands, and dependency state unchanged

#### Scenario: A concentration finding is clustered
- **WHEN** a knowledge-concentration finding exists
- **THEN** one `bus_risk` card carries its contributor count, numerator, and denominator and no contributor identity

#### Scenario: A file has one ordinary Watch finding
- **WHEN** no named pattern claims the file
- **THEN** one `measured` card names that file's top ranked finding and claims its remaining verdict-affecting findings

### Requirement: Descriptive cards carry no rating and stay out of default detail
A `hub` card whose file carries no rated finding and no size finding SHALL be
descriptive: its rating SHALL be `healthy`, it SHALL claim no finding, and it
SHALL be excluded from default terminal detail while remaining available in
`--all` and in the machine report. A descriptive card SHALL NOT affect any
verdict, count, or rank position of a rated card.

Every other pattern SHALL produce a rated card, because each of them requires a
rated or size finding to exist.

#### Scenario: A healthy utility is imported everywhere
- **WHEN** a file with 40 importers carries no rated finding and no size finding
- **THEN** its `hub` card is descriptive, carries rating `healthy`, and is absent from default terminal detail

#### Scenario: A widely imported file also carries debt
- **WHEN** a file that satisfies the `hub` rule carries a High finding
- **THEN** its card is rated High and appears in default detail

### Requirement: Problem rank is total and built once
Analysis SHALL order the problem table by rating descending, claimed High count
descending, hot before not hot, claimed finding count descending, pattern in the
frozen claiming order, the accepted finding rank of the card's top claimed
finding, anchor repository-relative path ascending, and anchor start line
ascending, in that order. This specification SHALL NOT restate the keys of the
finding rank, which `hotspot-analysis` owns.

The order SHALL be total and data-stable, so serial and parallel runs produce
identical card order. The table SHALL be sorted once when the report is
finished, and no renderer SHALL sort, re-rank, or reorder it.

#### Scenario: Two cards tie on rating and claimed counts
- **WHEN** two cards share rating, claimed High count, hot state, and claimed finding count
- **THEN** the frozen pattern order decides, and after it the accepted finding rank of each card's top claimed finding

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
