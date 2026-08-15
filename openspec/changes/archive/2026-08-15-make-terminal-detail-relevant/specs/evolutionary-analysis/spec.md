## MODIFIED Requirements

### Requirement: Unexplained recurring coupling is a Watch finding
The system SHALL create a Watch finding only when a pair has at least three
shared commits, Jaccard similarity of at least 0.20, sufficient history, and no
trusted eligible `uses` relation in either direction. Terminal default and
`--all` SHALL show only these actionable findings through existing
evolution-finding IDs. Weaker observations and coupling explained by code SHALL
remain visible in JSON and SHALL NOT appear in any human default, `--all`, or
path view. Each human finding SHALL retain exact shared commits, union commits,
similarity, and the final phrase `not linked in code`.

#### Scenario: A pair meets both thresholds
- **WHEN** a pair has 3 shared commits, 15 union commits, sufficient history, and no explaining use
- **THEN** one Watch finding exposes the exact operands and evidence in JSON and through its linked human finding row

#### Scenario: A pair misses one threshold
- **WHEN** a pair has two shared commits or similarity below 0.20
- **THEN** it remains available in JSON but is absent from default, `--all`, and path terminal views

#### Scenario: Trusted uses explain the pair
- **WHEN** an eligible parsed uses relation exists in either direction
- **THEN** coupling remains descriptive in JSON, creates no finding, and appears in no human history section

### Requirement: Diff reports use history only as context
Report and JSON SHALL retain existing history facts for changed files and
packages without treating them as before-and-after measurements. Human diff
views SHALL select history only through actionable evolution-finding IDs and
SHALL NOT render weak or explained observations, activity, concentration,
coverage fields, or processing facts. This requirement SHALL NOT change diff
direction or comparison policy.

#### Scenario: A worktree edits a high-churn package without an actionable history finding
- **WHEN** diff analysis retains package churn as context
- **THEN** JSON retains the context and human default, `--all`, and path views add no activity row

#### Scenario: A worktree has an actionable unexplained coupling finding
- **WHEN** its existing evolution-finding ID is linked to the selected diff scope
- **THEN** human history can show that finding with exact evidence and `not linked in code` without labeling unchanged history Better or Worse

### Requirement: History coverage and concentration fields are exact
History coverage SHALL expose stream availability, revision, total streamed
commits, commits containing eligible current source, mapped eligible changes,
mapped context changes, newest and oldest timestamps, textual changes,
uncounted changes, excluded changes, rename gaps, and reason in report and JSON.
A mapped fixture, generated, recovered, or failed change is context rather than
excluded. Textual plus uncounted changes SHALL equal mapped eligible plus
context changes, and eligible commits SHALL NOT exceed streamed commits.

Contributor concentration SHALL expose package ID, SourceRole, trust,
contributor count, numerator, denominator, and ratio in report and JSON without
identity. Eligible and context contributors SHALL remain separate. Human
default, `--all`, and path views SHALL NOT render history coverage fields,
activity rows, or contributor concentration rows. A commit count already
attached to a selected source finding SHALL remain valid finding evidence.

#### Scenario: History is shallow
- **WHEN** only part of repository history is locally available
- **THEN** JSON keeps every exact field and human output uses only the grouped incomplete-history warning

#### Scenario: Generated history dominates a package
- **WHEN** many generated contributors touch a package and few eligible parsed commits touch it
- **THEN** JSON retains both evidence rows while every human view omits contributor concentration and package activity rows

### Requirement: Stream and mapping evidence are presented separately
Report and JSON SHALL retain stream completeness separately from eligible
mapping counts and percentage. A complete Git stream alone SHALL NOT be treated
as sufficient architecture evidence. Human default, `--all`, and path views
SHALL NOT render stream totals, mapping totals, mapping percentages, eligible
commit totals, or processing evidence. When history is incomplete, the grouped
human history warning SHALL remain.

#### Scenario: Most observed changes are context or excluded
- **WHEN** Git streaming is complete and only some changes map to eligible current source
- **THEN** JSON reports separate exact stream and mapping facts while human output adds no stream or mapping row

### Requirement: Default history presentation does not repeat facts
Every human history section SHALL select rows only through the selected scope's
existing actionable evolution-finding IDs. Default SHALL take at most the first
three in stable presentation order and `--all` SHALL take every linked ID. Path
selection SHALL NOT add weak, explained, activity, concentration, coverage, or
processing facts. Analysis/report aggregation SHALL keep actionable IDs unique
within each owning scope list, and index audits SHALL reject duplicates.
Terminal history SHALL trust the list and visit each ID once without a de-dup
set, second collection, or identity mapping. Descriptive/context tables SHALL
never be selected. Each actionable finding SHALL end with `not linked in code`.

#### Scenario: A coupling finding appears at one selected scope
- **WHEN** default, `--all`, and path views are compared
- **THEN** each unique actionable ID is visited once and no indistinguishable activity, package, or contextual history row is selected

#### Scenario: An actionable list repeats an ID
- **WHEN** report/index integrity is audited
- **THEN** the audit fails before terminal history selection rather than hiding the duplicate during rendering
