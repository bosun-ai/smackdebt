## MODIFIED Requirements

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
