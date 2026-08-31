## ADDED Requirements

### Requirement: A verdict states how far a change propagates
Analysis SHALL carry two propagation facts on a completed verdict, each with a
frozen sentence analysis owns so every consumer prints identical bytes, and no
renderer SHALL compose, recompute, or reword one:

- **Propagation reach.** At the repository root the sentence SHALL be
  `A change in one package can reach 9 of 14 packages.` with the reached and
  total package counts substituted. At a package scope the sentence SHALL be
  `A change here can reach 34 of 98 files in this package.` with the reached and
  total file counts substituted. The fact SHALL retain both integers.
- **Core size.** At the repository root the sentence SHALL be
  `34 of 210 files sit in one dependency cycle.` with the core and graph file
  counts substituted. The fact SHALL retain both integers.

Both file counts are the files the dependency graph is built over — the scope's
primary, parsed files — because a fraction whose halves come from two
populations answers nothing. A package's test, example, benchmark, fixture, and
generated files are therefore outside both halves, so on a real package the
denominator reads lower than the file count that package holds: measured gaps
run from a tenth to a half of a package's files. The documentation SHALL state
which files these sentences count, for the sentences and for the JSON members
that carry the same integers, so a reader who counts their own package finds the
rule rather than a contradiction.

Each fact SHALL be present only where its scope and its materiality rule allow
it: reach at the repository root and at a package scope, core size at the
repository root only, and neither at a directory or file scope. Where the
`architecture-analysis` materiality rule that owns the underlying number does not
hold, the fact SHALL be absent rather than stated as a zero, a one-of-one, or a
hedge. The numbers SHALL be written as plain digits without grouping, as the
accepted repository-share sentence writes them.

A repository that holds exactly one package **is** that package, and no view can
select that package's own scope, because the repository path every consumer
writes as `.` selects the repository. Where such a repository's package-reach
fact is immaterial — which it always is, one package being below the package
floor — and that one package's file-reach fact is material, the repository root
SHALL state the package-scope sentence rather than nothing. The sentence is the
accepted one and no wording is invented for the case: the scope the reader
selected is the package the number is about, so `A change here` names the same
tree either way. Without this the most common shape there is, one library or one
application in one repository, would be the only shape that can never state how
far a change reaches.

A repository holding more than one package SHALL NOT borrow a package's sentence
for its root, however few of its packages are material, because there the root
is not the package and `in this package` would name a scope the reader did not
select.

Both facts SHALL be stated only. They SHALL NOT change the selected tier, the
counts behind it, the worst offender, or any rating, exactly as the
unsupported-coverage qualifier and the repository-share fact never do. Producing
them SHALL read only the completed report and perform no filesystem, Git,
parser, or analysis work.

#### Scenario: A root verdict carries both facts
- **WHEN** the repository root of a layered repository with a large core is selected
- **THEN** the verdict carries `A change in one package can reach 9 of 14 packages.` and `34 of 210 files sit in one dependency cycle.` with their integer operands

#### Scenario: A package verdict carries reach
- **WHEN** a package holding 98 files is selected and one of its files is depended on by 33 others
- **THEN** the verdict carries `A change here can reach 34 of 98 files in this package.` and carries no core size fact

#### Scenario: A single-package repository states its file reach at its root
- **WHEN** the repository root of a repository declaring one package of 98 files is selected and one of its files is depended on by 33 others
- **THEN** the verdict carries `A change here can reach 34 of 98 files in this package.`

#### Scenario: A single-package repository is below the file floor
- **WHEN** the repository declares one package holding fewer files than the file floor
- **THEN** the verdict carries no reach fact

#### Scenario: A multi-package repository never borrows a package sentence
- **WHEN** a repository of several packages has no cross-package dependency and exactly one of its packages has a material file reach
- **THEN** the root verdict carries no reach fact and that package's own scope still states its sentence

#### Scenario: A directory scope is selected
- **WHEN** a directory below a package is selected
- **THEN** the verdict carries neither propagation fact

#### Scenario: The facts never move the tier
- **WHEN** the same scope verdict is computed with and without both propagation facts available
- **THEN** the tier, its counts, and the worst offender are identical

#### Scenario: Two consumers print a propagation fact
- **WHEN** the same report is rendered for a human and serialized for a machine
- **THEN** both carry the analysis-owned sentence bytes and the same integer operands

### Requirement: A verdict states what a typical change costs
Analysis SHALL carry a change-amplification fact on the verdict of a repository,
package, or directory scope, with the frozen sentence
`A typical change here touches 4 files.` and its median substituted, retaining
the median and the commit count it was computed from as integers.

The fact SHALL be absent at a file scope and absent whenever the materiality
rule `change-leakage` owns does not hold, so a scope with too little history or
a median below the floor states nothing rather than stating noise. This
specification SHALL NOT restate that rule.

The fact SHALL be stated only: it SHALL NOT change the selected tier, the counts
behind it, the worst offender, or any rating, and it SHALL be owned by analysis
so every consumer prints identical bytes.

#### Scenario: A directory has a typical change size
- **WHEN** a directory was touched by 40 commits whose nearest-rank median file count is 4
- **THEN** its verdict carries `A typical change here touches 4 files.` with both integers retained

#### Scenario: A file scope is selected
- **WHEN** the selected scope is a file
- **THEN** its verdict carries no amplification fact

#### Scenario: The fact never moves the tier
- **WHEN** the same scope verdict is computed with and without the amplification fact available
- **THEN** the tier, its counts, and the worst offender are identical

#### Scenario: A scope's typical change is one file
- **WHEN** a scope has enough commits but its median file count is below the floor
- **THEN** its verdict carries no amplification fact rather than a sentence stating a median of one or two
