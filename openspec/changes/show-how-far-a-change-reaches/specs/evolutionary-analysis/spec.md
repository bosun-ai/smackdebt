## ADDED Requirements

### Requirement: File change coupling rides the one history stream
File change coupling and change amplification SHALL be accumulated during the
existing streamed history process, as further members of the one accumulator
that already fans a commit out to churn, package coupling, and contributor
concentration. They SHALL NOT start a second Git process, re-read history, or
retain per-commit file lists after the commit that produced them, because the
stream delivers one commit at a time by design.

A retained file pair SHALL keep its two file identities with the lower-indexed
file first, its shared commit count, its union commit count, and the integer
directory distance between the two files. Rename identity SHALL follow the
accepted rule: history joins a current file only through a parsed rename chain,
so a broken chain under-reports a pair rather than inventing one, and the
existing rename-gap disclosure covers it.

Every new table SHALL be ordered by data-stable keys so serial and parallel runs
produce identical bytes. The guards, retention thresholds, and detector rules
belong to `change-leakage`, and this specification SHALL NOT restate them.

#### Scenario: One report reads history once
- **WHEN** a codebase report accumulates churn, package coupling, concentration, file pairs, and amplification
- **THEN** exactly one history process supplies all of them

#### Scenario: A file was renamed inside the window
- **WHEN** a file's rename chain is unbroken across the window
- **THEN** its earlier paths' commits count toward its pairs under its current identity

#### Scenario: A rename chain is broken
- **WHEN** an earlier path cannot be linked uniquely to a current file
- **THEN** the affected commits contribute no pair and the existing rename-gap disclosure states the exclusion

#### Scenario: Worker count changes
- **WHEN** the same generated history fixture is analyzed with one worker and automatic parallelism
- **THEN** the file pair table, the amplification facts, and the leakage findings are identical in content and order

## MODIFIED Requirements

### Requirement: The history window governs every history signal
When a history window is selected, the window SHALL be applied when streamed
history records become facts, so touches, churn, package change coupling, file
change coupling, change amplification, and contributor concentration all
describe the same window. No history-derived value SHALL be computed over
commits outside the selected window. The window SHALL still be read from one
streamed history process per report.

The window filter SHALL be applied inside the streamed history process itself,
so history outside the window is never streamed, and a defensive in-process
boundary check SHALL remain so no out-of-window record can become a fact. Both
the process filter and the boundary check SHALL compare the same instant: the
moment a change landed on the analyzed history — the committer date — not the
moment it was authored. A rebased or cherry-picked commit therefore counts by
when it landed, and the two filters can never silently disagree.

#### Scenario: A window is selected
- **WHEN** a report is requested with a history window that excludes older commits
- **THEN** touches, churn, coupling operands, file pair operands, amplification observations, and concentration operands all exclude those commits

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
length in days when a window is selected, the number of streamed commits
excluded by that window, the number of commits the bulk-commit guard excluded
from file pair accumulation, and the number of file pairs the storage limit
declined. When the window filter runs inside the streamed
history process, streamed commits, newest timestamp, and oldest timestamp
SHALL describe the windowed set, and the window-excluded count SHALL count
boundary rejects from the defensive in-process check, normally zero. Commits
excluded by the window SHALL be counted separately from changes excluded for
other reasons. A mapped fixture, generated, recovered, or failed change is
context rather than excluded. Textual plus uncounted changes SHALL equal
mapped eligible plus context changes, and eligible commits SHALL NOT exceed
streamed commits.

The two change-graph counters SHALL describe pair accumulation only. A
bulk-excluded commit SHALL remain a fully counted commit everywhere else, so it
still contributes churn, touches, package coupling, concentration, and one
amplification observation. Both counters SHALL be added through their own
builder, mirroring the accepted window builder, so the existing constructor is
unchanged.

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

#### Scenario: A bulk commit is excluded from pairs
- **WHEN** one commit exceeds the bulk-commit guard
- **THEN** coverage states one bulk commit while that commit's churn, touches, package coupling, concentration, and amplification observation are unchanged

#### Scenario: Concentration is serialized
- **WHEN** a concentration row is emitted in the machine report
- **THEN** its numerator and denominator are present and no ratio value is serialized
