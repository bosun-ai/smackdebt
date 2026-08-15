## MODIFIED Requirements

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

### Requirement: History coverage and concentration fields are exact
History coverage SHALL expose stream availability, revision, total streamed commits,
commits containing eligible current source, mapped eligible changes, mapped
context changes, newest and oldest timestamps, textual changes, uncounted
changes, excluded changes, rename gaps, reason, the selected history window
length in days when a window is selected, and the number of streamed commits
excluded by that window. Commits excluded by the window SHALL be counted
separately from changes excluded for other reasons. A mapped fixture, generated,
recovered, or failed change is context rather than excluded. Textual plus
uncounted changes SHALL equal mapped eligible plus context changes, and eligible
commits SHALL NOT exceed streamed commits.

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
- **THEN** coverage states the window length in days and the exact number of streamed commits the window excluded

#### Scenario: Concentration is serialized
- **WHEN** a concentration row is emitted in the machine report
- **THEN** its numerator and denominator are present and no ratio value is serialized
