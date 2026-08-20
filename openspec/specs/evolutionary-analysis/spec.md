# evolutionary-analysis Specification

## Purpose
TBD - created by archiving change add-evolutionary-architecture-analysis. Update Purpose after archive.
## Requirements
### Requirement: History is streamed once into compact facts

The system SHALL read locally available non-merge history through one streamed
Git process per report and retain only the facts required for aggregation.

#### Scenario: One report reads history

- **WHEN** a codebase report analyzes a repository with many commits
- **THEN** exactly one history process supplies commit, rename, churn, and
  normalized contributor facts
- **AND** full commit messages and source contents are not retained

#### Scenario: A binary change is present

- **WHEN** Git reports a changed file without textual line counts
- **THEN** the change contributes a file and package touch
- **AND** the system does not invent added or deleted line counts

### Requirement: Current file identity follows supported renames

The system SHALL attribute older history to a selected current file only when a
parsed rename chain establishes that identity.

#### Scenario: A file was renamed

- **WHEN** a selected current file has an unbroken rename chain
- **THEN** touches and textual churn from its earlier paths belong to that
  current file

#### Scenario: A rename chain cannot be trusted

- **WHEN** an earlier path cannot be linked uniquely to a selected current file
- **THEN** the system excludes that record from current file measurements
- **AND** history coverage reports the exclusion

### Requirement: Churn and touches use exact aggregation rules
File and package history SHALL expose `touches`, `added_lines`, `deleted_lines`,
and `uncounted_changes`. A package SHALL receive at most one touch per commit,
textual lines SHALL be summed exactly, and binary changes SHALL increase
uncounted changes without invented line counts. Rows SHALL retain SourceRole and
trust after aggregation. Package history SHALL keep separate evidence rows for
eligible parsed source and context source instead of merging their operands.

#### Scenario: One commit changes text and a binary file
- **WHEN** both changes belong to one package
- **THEN** the package gains one touch, exact textual lines, and one uncounted change

### Requirement: Package change coupling is explainable
The system SHALL retain left and right package IDs, `shared_commits`, and `union_commits`
for each retained unordered pair, and SHALL derive Jaccard similarity from those
operands for threshold evaluation and human presentation. Similarity SHALL NOT
be serialized in the machine report; a consumer SHALL derive it from the
operands at its own precision. Each unordered package pair SHALL produce exactly
one retained coupling row; SourceRole and trust variants SHALL be aggregated
into that row rather than emitted as sibling rows, and per-role evidence SHALL
remain in package history rows. Eligible parsed roles SHALL feed one
package-pair aggregate used for findings, so fixture, generated, recovered, and
failed history cannot change finding operands.

A pair SHALL be excluded before similarity is derived when one endpoint scope is
an ancestor of the other, because shared commits between a scope and its own
descendant are structural rather than evidence of hidden coupling.

Each retained pair SHALL additionally carry a link classification computed
once when the report is built: `direct` when a trusted eligible `uses`
relation links the pair in either direction, `indirect` when no direct
relation exists but a dependency path connects the pair through the package
graph in either direction — naming the first intermediate package on a
shortest such path — and `none` when no path exists. The classification SHALL
inform presentation only: finding creation SHALL be unchanged, so an indirect
link does not suppress or explain an unexplained-coupling Watch finding.

#### Scenario: Two packages change together
- **WHEN** packages share three commits and fifteen commits touch either package
- **THEN** the row exposes shared count 3 and union count 15 and human output can state 20%

#### Scenario: One commit changes several files per package
- **WHEN** the same pair occurs several times inside one commit
- **THEN** that commit adds one shared change to the pair

#### Scenario: One pair has several role and trust variants
- **WHEN** a package pair is touched by primary, test, and generated source
- **THEN** exactly one coupling row exists for that pair with one set of operands, and no view can print the same pair with different numbers

#### Scenario: One scope contains the other
- **WHEN** the repository root scope and one of its packages change in the same commits
- **THEN** no coupling row and no coupling finding exists for that pair

#### Scenario: A pair is linked through an intermediate package
- **WHEN** package a depends on b and b depends on c, and a and c change together without a direct relation
- **THEN** the a–c pair is classified indirect naming b, and its Watch finding is created exactly as before

### Requirement: Unexplained recurring coupling is a Watch finding
The system SHALL create a Watch finding only when a pair has at least three
shared commits, Jaccard similarity of at least 0.20, sufficient history, no
ancestor-descendant relationship between its endpoints, and no trusted eligible
`uses` relation in either direction. Trusted eligible `uses` SHALL mean parsed
`uses` from primary, test, example, or benchmark source, independently of whether
that relation enters an architecture verdict graph. A `uses` relation resolved
through a unique manifest-name match SHALL explain a pair exactly as a
path-resolved relation does. Weaker observations SHALL remain visible in JSON and
`--all` and SHALL NOT appear as default findings.

#### Scenario: A pair meets both thresholds
- **WHEN** a pair has 3 shared commits, 15 union commits, sufficient history, and no explaining use
- **THEN** one Watch finding exposes the exact operands and absent relationship

#### Scenario: A pair misses one threshold
- **WHEN** a pair has two shared commits or similarity below 0.20
- **THEN** it is absent from default findings but remains available in JSON and `--all`

#### Scenario: Trusted uses explain the pair
- **WHEN** an eligible parsed uses relation exists in either direction
- **THEN** coupling remains descriptive and creates no finding

#### Scenario: A test-role dependency explains the pair
- **WHEN** two packages are linked only by a trusted test-role `uses` relation, such as a dev-dependency import in a test file or in a `#[cfg(test)]` module
- **THEN** their coupling is explained, no Watch finding is created, and no output claims `no code dependency` for that pair, even though that relation never enters an architecture verdict graph

#### Scenario: A manifest-name edge explains the pair
- **WHEN** two workspace packages import each other only by declared manifest name
- **THEN** their coupling is explained, no Watch finding is created, and no output claims `no code dependency` for that pair

### Requirement: Contributor concentration protects identity

The system SHALL report contributor count and top-contributor concentration
without retaining or emitting contributor identity.

#### Scenario: Several contributors touch one package

- **WHEN** normalized contributors have package touch counts 3, 2, and 1
- **THEN** the package reports contributor count 3, numerator 3, denominator 6,
  and concentration 0.5
- **AND** no name, address, raw author field, or internal contributor identifier
  enters terminal or JSON output

### Requirement: History limitations remain visible

The system SHALL preserve source and static architecture results when history is
unavailable or incomplete and SHALL attach an exact history status and
diagnostic. A selected window containing zero commits over a complete stream
SHALL be a stated limitation: the report SHALL disclose the empty window
rather than staying silent, and source and architecture results SHALL remain.

#### Scenario: The selected directory has no Git history

- **WHEN** source analysis succeeds outside a Git repository
- **THEN** the report contains source and static architecture results
- **AND** evolution is unavailable with a diagnostic

#### Scenario: The repository is shallow

- **WHEN** only shallow local history is available
- **THEN** the report identifies the analyzed window as incomplete
- **AND** does not describe the values as complete repository history

#### Scenario: Every commit is older than the window

- **WHEN** the stream is complete and zero commits fall inside the selected window
- **THEN** the report discloses the empty window
- **AND** source and static architecture results remain

### Requirement: Diff reports use history only as context

The system SHALL show existing history facts for changed files and packages
without treating those facts as before-and-after measurements.

#### Scenario: A worktree edits a high-churn package

- **WHEN** diff analysis changes source inside a package with retained history
- **THEN** the package's churn and coupling appear as context
- **AND** unchanged historical values are not labelled Better or Worse

#### Scenario: A worktree adds an explaining static edge

- **WHEN** a static edge introduced by the worktree explains an existing
  coupling finding
- **THEN** the evolutionary finding is reported as removed
- **AND** the historical coupling values remain unchanged

### Requirement: History coverage and concentration fields are exact
History coverage SHALL expose stream availability, revision, total streamed commits,
commits containing eligible current source, mapped eligible changes, mapped
context changes, newest and oldest timestamps, textual changes, uncounted
changes, excluded changes, rename gaps, reason, the selected history window
length in days when a window is selected, and the number of streamed commits
excluded by that window. When the window filter runs inside the streamed
history process, streamed commits, newest timestamp, and oldest timestamp
SHALL describe the windowed set, and the window-excluded count SHALL count
boundary rejects from the defensive in-process check, normally zero. Commits
excluded by the window SHALL be counted separately from changes excluded for
other reasons. A mapped fixture, generated, recovered, or failed change is
context rather than excluded. Textual plus uncounted changes SHALL equal
mapped eligible plus context changes, and eligible commits SHALL NOT exceed
streamed commits.

Contributor concentration SHALL expose package ID, SourceRole, trust,
contributor count, numerator, and denominator without identity. The
concentration ratio SHALL be derived from the numerator and denominator for
threshold evaluation and human presentation and SHALL NOT be serialized in the
machine report. Eligible and context contributors SHALL remain in separate rows
so context contributors cannot alter eligible top share.

#### Scenario: History is shallow
- **WHEN** only part of repository history is locally available
- **THEN** every field remains explicit and reason states incomplete history

#### Scenario: A complete stream has no eligible current source
- **WHEN** Git streaming completes but zero commits and changes map to eligible current source
- **THEN** history observations remain descriptive and no evolutionary finding is created

#### Scenario: Generated history dominates a package
- **WHEN** many generated contributors touch a package and few eligible parsed commits touch it
- **THEN** JSON and `--all` retain both evidence rows while default churn and top share use only the eligible row

#### Scenario: A window excludes older commits
- **WHEN** a report is requested with a history window
- **THEN** coverage states the window length in days and describes the windowed set, with the window-excluded count carrying only boundary rejects

#### Scenario: Concentration is serialized
- **WHEN** a concentration row is emitted in the machine report
- **THEN** its numerator and denominator are present and no ratio value is serialized

### Requirement: Stream and mapping evidence are presented separately
Terminal history coverage SHALL state stream completeness separately from
eligible mapping counts and percentage. A complete Git stream alone SHALL NOT
be described as sufficient architecture evidence.

#### Scenario: Most observed changes are context or excluded
- **WHEN** Git streaming is complete and only some changes map to eligible current source
- **THEN** terminal output reports complete stream evidence and the eligible mapped count, total observed count, percentage, and eligible commit count separately

### Requirement: Default history presentation does not repeat facts
Default terminal output SHALL present one package history row, coupling pair, or
contributor concentration at its nearest useful scope at most once. `--all` and
path drill MAY expand evidence but SHALL NOT duplicate an identical row within
one rendered view.

#### Scenario: A coupling finding appears with package history
- **WHEN** default output includes both sections
- **THEN** its shared and union operands are printed once rather than repeated in summary and detail

### Requirement: The history window governs every history signal
When a history window is selected, the window SHALL be applied when streamed
history records become facts, so touches, churn, package change coupling, and
contributor concentration all describe the same window. No history-derived value
SHALL be computed over commits outside the selected window. The window SHALL
still be read from one streamed history process per report.

The window filter SHALL be applied inside the streamed history process itself,
so history outside the window is never streamed, and a defensive in-process
boundary check SHALL remain so no out-of-window record can become a fact. Both
the process filter and the boundary check SHALL compare the same instant: the
moment a change landed on the analyzed history — the committer date — not the
moment it was authored. A rebased or cherry-picked commit therefore counts by
when it landed, and the two filters can never silently disagree.

#### Scenario: A window is selected
- **WHEN** a report is requested with a history window that excludes older commits
- **THEN** touches, churn, coupling operands, and concentration operands all exclude those commits

#### Scenario: No window is selected
- **WHEN** no history window is requested
- **THEN** every history signal uses all locally available non-merge history

#### Scenario: An old commit is rebased recently
- **WHEN** a change authored outside the window landed inside it
- **THEN** the commit counts inside the window under both filters

#### Scenario: A narrow window is requested on a long history
- **WHEN** a report is requested with a window covering a small fraction of history
- **THEN** commits outside the window never reach the report and are not streamed into the process output

### Requirement: Concentrated knowledge is a Watch finding
The system SHALL create a Watch evolutionary finding for a package when it has
at least 10 commits within the analyzed window and its top contributor share is
at least 90%. The share comparison SHALL use integer numerator and denominator
operands. The finding SHALL retain package identity, contributor count,
numerator, and denominator only, and SHALL NOT retain or emit any contributor
name, address, raw author field, or internal contributor identifier. Weaker
observations SHALL remain descriptive.

Evolutionary findings SHALL carry a kind that distinguishes unexplained coupling
from knowledge concentration.

#### Scenario: One contributor owns a package
- **WHEN** a package has 20 windowed commits and one contributor accounts for 19 of them
- **THEN** one Watch finding retains contributor count, numerator 19, and denominator 20 without any identity

#### Scenario: A package is just below the thresholds
- **WHEN** a package has 9 windowed commits, or a top share of 89%
- **THEN** no knowledge-concentration finding is created and the observation stays descriptive

#### Scenario: Finding kinds are inspected
- **WHEN** a report contains both an unexplained coupling finding and a knowledge-concentration finding
- **THEN** each finding states its kind and the two remain distinguishable
