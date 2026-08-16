## ADDED Requirements

### Requirement: The history window governs every history signal
When a history window is selected, the window SHALL be applied when streamed
history records become facts, so touches, churn, package change coupling, and
contributor concentration all describe the same window. No history-derived value
SHALL be computed over commits outside the selected window. The window SHALL
still be read from one streamed history process per report.

#### Scenario: A window is selected
- **WHEN** a report is requested with a history window that excludes older commits
- **THEN** touches, churn, coupling operands, and concentration operands all exclude those commits

#### Scenario: No window is selected
- **WHEN** no history window is requested
- **THEN** every history signal uses all locally available non-merge history

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

## MODIFIED Requirements

### Requirement: History coverage and concentration fields are exact
History coverage SHALL expose stream availability, revision, total streamed
commits, commits containing eligible current source, mapped eligible changes,
mapped context changes, newest and oldest timestamps, textual changes,
uncounted changes, excluded changes, rename gaps, reason, the selected history
window length in days when a window is selected, and the number of streamed
commits excluded by that window. Commits excluded by the window SHALL be counted
separately from changes excluded for other reasons. A mapped fixture,
generated, recovered, or failed change is context rather than excluded.
Textual plus uncounted changes SHALL equal mapped eligible plus context changes,
and eligible commits SHALL NOT exceed streamed commits.

Contributor concentration SHALL expose package ID, SourceRole, trust,
contributor count, numerator, denominator, and ratio without identity. Eligible
and context contributors SHALL remain in separate rows so context contributors
cannot alter eligible top share.

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
