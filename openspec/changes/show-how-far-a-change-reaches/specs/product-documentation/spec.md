## MODIFIED Requirements

### Requirement: README explains problem cards
The README SHALL explain that a codebase report names problems rather than
listing measurements: that a problem card groups findings the report already
produced around one file, cycle, package, or package pair, that each finding
belongs to at most one card so a file with several findings is named once, and
that clustering invents no measurement, rating, or verdict.

It SHALL list the frozen pattern ids `god_file`, `hub`, `tangle`, `hot_mess`,
`shotgun_pair`, `bus_risk`, `unstable_dependency`, `measured`,
`leaky_interface`, and `hidden_coupling` beside the
words the terminal prints for each, explain in product language what evidence
each one needs, and publish the threshold behind every named pattern the way the
existing rating thresholds are published. It SHALL print `packages change
together` as the words for `shotgun_pair`, and SHALL NOT keep the earlier
`changes together` wording, which no longer says whether packages or files are
meant. It SHALL explain that `measured` is
the fallback so nothing rated disappears, that every card is either a `default`
card or a `detail` card shown only under `--all`, at its own anchor, and in
JSON — a widely imported file with no debt of its own and no leakage finding
naming it, a file whose whole debt is advisory or non-primary, and a file whose
only evidence is its size are all `detail` cards — and that cards are ranked
against each other so the first card is the problem to look at first.

It SHALL state the qualification plainly rather than leave the example
contradicting the rule: the same widely imported file becomes a `default` card
as soon as a change-leakage finding names it, because a file whose importers
follow its changes is a problem even when the file itself measures clean.

It SHALL explain that a file's leakage numbers appear as extra lines on the card
that already names that file, and that a card of their own exists only when
nothing else names the file or the pair, so the report never states one subject
twice.

It SHALL document the repository-share sentence in a package, directory, or file
verdict, state that it is absent at the repository root, and explain that it
frames how much of the whole problem the selected scope holds.

#### Scenario: A user sees a named problem
- **WHEN** the user reads a card in their own report and looks it up
- **THEN** the README names the pattern, states the evidence it needs, and explains what to do about it

#### Scenario: A user misses their per-finding rows
- **WHEN** a user who knew the old finding list reads the new documentation
- **THEN** it explains that one card now claims that file's findings and that JSON retains each of them

#### Scenario: A user reads about a clean file that carries a card
- **WHEN** the user reads the `detail` card examples after seeing a widely imported file in their default view
- **THEN** the README explains that the file is a `detail` card only while nothing names it, and that a leakage finding makes the same card `default`

#### Scenario: A user reads about two things that change together
- **WHEN** the user compares the `shotgun_pair` and `hidden_coupling` entries
- **THEN** the README says plainly that one is about packages and the other about files, and names the words the terminal prints for each

#### Scenario: A user drills into a package
- **WHEN** the user reads the sub-scope example
- **THEN** the README explains the share sentence and why the repository root does not print one

### Requirement: README states JSON detail retention
The README SHALL state that JSON retains every Watch and High finding and all scope
summaries, while healthy units are represented through aggregate counts. It
SHALL also state that JSON is the complete view of every fact the terminal
omits, including raw dependency edges, external references, churn detail, weak
coupling, weak file change coupling, hotspots, size findings, orphan files,
file change coupling, package closures, candidate file reach, and every problem
card including `detail` ones.

It SHALL state that dependency edges appear in no human view at any scope or
detail level, that the terminal states a relationship only as aggregate problem
evidence such as a fan-in count or a cycle witness, and that the JSON relation
tables are therefore the only place to read the edges themselves.

It SHALL state that a retained file change-coupling pair that produced no
finding reaches no human view at all, `--all` included, and that the JSON table
is where a reader inspects the weaker pairs.

#### Scenario: Integration author chooses JSON output
- **WHEN** an integration needs all debt findings
- **THEN** the README makes clear that JSON is complete for Watch and High findings but not a full healthy-unit index

#### Scenario: A user misses a row the terminal removed
- **WHEN** they look for a row that human output no longer prints
- **THEN** the README directs them to the version-4 JSON table that retains it

#### Scenario: A user looks for weak file pairs
- **WHEN** they read the JSON documentation after seeing one leakage card and expecting more
- **THEN** the README states that weaker pairs are JSON-only by design and names the table

#### Scenario: A user looks for import rows
- **WHEN** they read the architecture or JSON documentation after their import rows disappeared
- **THEN** the README states that edges are JSON-only and shows the card evidence that replaced them

## ADDED Requirements

### Requirement: README explains how far a change reaches
The README SHALL explain, in product language, the three sentences a codebase
verdict may carry beyond its tier: how far a change in one package can reach,
how far a change inside the selected package can reach, how much of the codebase
sits in one dependency cycle, and how many files a typical change here touches.
It SHALL state that each one is a stated fact that never changes the tier, the
counts, or the worst offender, and that each is absent — rather than hedged —
when the repository is too small, the history too thin, or the number too weak
to mean anything.

It SHALL explain the two co-change patterns in the same terms it already
explains package coupling: that a leaky interface is a file whose importers keep
changing with it across a directory boundary, that hidden coupling is a pair of
files that change together where no dependency path connects them in either
direction, that both require the two files to sit in different directories, and
that a larger directory distance lowers the agreement each needs because
distance is what makes co-change surprising. It SHALL publish the support,
distance, and similarity thresholds the way the existing coupling thresholds are
published, and SHALL state that a hidden pair is reported only when the absence
of a path was proved, so an inconclusive search reports nothing.

It SHALL state which files the two file counts count — the scope's primary,
parsed files, the population every dependency fact in the report uses — so a
reader whose package holds more files than the sentence names finds the rule
rather than a contradiction. It SHALL state that a leaky interface is never a
wiring file, because a file that is a list of declarations and re-exports has no
abstraction to leak and its importers change with it by construction, and it
SHALL name those files rather than implying that every conventional entry file
is one.

It SHALL state that these signals are derived from history and therefore never
enter the ratchet gate. It SHALL explain that trusted reach, core-size, and
change-leakage movement can appear in a diff report while change amplification
remains codebase-only.

#### Scenario: A user reads the verdict head
- **WHEN** the user sees a sentence about how far a change reaches
- **THEN** the README explains what the two numbers count, why the sentence may be absent, and that it does not affect the verdict

#### Scenario: A user sees a leakage card
- **WHEN** the user looks up `importers follow its changes` or `change together without a dependency`
- **THEN** the README explains the evidence each needs, publishes the thresholds, and explains why nothing is reported when a path search is inconclusive

#### Scenario: A user counts the files in their own package
- **WHEN** a package holds more files than the reach sentence names
- **THEN** the README states that the sentence counts the package's primary, parsed files, the same population the dependency graph is built over

#### Scenario: A user asks why the gate ignores these signals
- **WHEN** the user reads the gate documentation beside the new signals
- **THEN** the README explains that history-derived signals move with wall-clock time and are excluded by rule

#### Scenario: A user reads a diff report
- **WHEN** reach, core size, or change leakage changed between two trusted graph sides
- **THEN** the README explains that the movement can appear in the architecture comparison family and that change amplification has no diff comparison
