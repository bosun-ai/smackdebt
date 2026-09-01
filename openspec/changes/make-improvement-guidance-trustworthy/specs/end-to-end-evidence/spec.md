## ADDED Requirements

### Requirement: Trustworthy guidance has exact generated acceptance
Privacy-safe generated repositories SHALL prove every revised decision through
the real CLI process in terminal and JSON mode. The fixture set SHALL include:

- supported, Astro, non-source, missing, and source-free explicit paths;
- root and diff inventories mixing supported and Astro source;
- one unchanged moved anonymous callback, one moved-and-edited callback with a
  stable semantic anchor, one genuine addition, one genuine deletion,
  repeated-current-only and repeated-base-only groups, and one two-to-one
  shared collision group;
- every generated JavaScript filename shape, both exact content edges, a large
  ordinary multiline file, a small authored one-line file, and authored source
  under `public`, `share`, and `assets`;
- source, architecture, and history rows spanning Worse, Better, and Changed;
- each neutral diff tier sentence, default selection, `--top 1`, `--all`, a
  fully checked verdict-only no-debt diff, a warning-bearing no-debt diff, and a
  documentation-only no-debt diff with retained current history context, and a
  non-empty diff footer. The warning-bearing case SHALL use a comparison-trust
  warning rather than unrelated current-state context.

Acceptance SHALL assert status, stream placement, exact relevant bytes, JSON
schema, index integrity, selected and analyzed file counts, unsupported and
failed counts, graph evidence, comparison direction, diagnostic grouping,
absence of private match values, and serial/automatic byte equality. It SHALL
assert that existing metric thresholds, graph thresholds, tier identifiers,
comparison directions, and genuine Rust cycle witnesses do not move.

#### Scenario: Explicit scopes are invoked
- **WHEN** the CLI receives each generated supported, unsupported, empty-directory, non-source, and missing path
- **THEN** each invocation has the exact selected scope or exact status, empty stdout, and stderr required by its path class without repository fallback

#### Scenario: Anonymous callbacks are compared
- **WHEN** the CLI diffs the generated callback history
- **THEN** the unchanged move is absent, the moved-and-edited callback is one comparison, one-sided groups retain every addition or removal, and only the shared collision group produces one scope warning counting its file once

#### Scenario: Generated JavaScript is inspected
- **WHEN** the generated-role fixture is rendered at root and at each file
- **THEN** generated files remain machine and file-detail context, authored files remain primary, and generated files own no default verdict, problem, worst offender, or navigation target

#### Scenario: A mixed diff is limited
- **WHEN** the default, `--top 1`, and `--all` terminal views are rendered
- **THEN** default represents each present direction, top one emits one ranked row, all emits every useful row, and every non-empty view ends with the exact detail footer

### Requirement: Real repositories prove that guidance is useful
Before implementation task 1.1, the current release binary SHALL capture the
complete matrix below in terminal and JSON. After generated acceptance passes,
one newly built release binary SHALL run the same matrix against that saved
pre-change evidence:

- Smackdebt root and `diff d4e78ba`;
- Fluyt root and `diff master`, including `WorkflowRunMiniMap.vue`;
- Fluyt `GraphEditor.vue` at current `f42f3ae8e76241e06254b7787a87a2f198fb70c2`
  against base `02caca0d3bf05599e1232130cf9265cc9926c9b3`;
- the marketing repository root, one Astro file, its Astro directory, and
  `diff HEAD`;
- Netdisco root and `diff HEAD~1`;
- Swiftide root and `diff HEAD~1`;
- Parity `diff HEAD~1`.

Every run SHALL inspect terminal and JSON from the same release build and record
selected, analyzed, unsupported, and failed file counts; graph status; emitted
and suppressed architecture comparisons; visible directions; footer presence;
and exit status. The review SHALL check that the terminal conclusion follows
from visible evidence, the selected scope is exact, generated context does not
crowd out authored work, anonymous changes are paired only with safe evidence,
and precise existing findings remain stable.

Committed review evidence SHALL contain no private source, contributor identity,
raw Git history, secret, or complete private terminal output. A failed or
unexplained check SHALL keep the related task open rather than being accepted as
a baseline. Commands, report digests, aggregate file and graph counts, visible
directions, footer presence, and exit status for the entire matrix SHALL be
recorded as a completion note beneath task 0.1 before implementation begins.

#### Scenario: Smackdebt and Fluyt callbacks are reviewed
- **WHEN** the release binary runs over the recorded diffs
- **THEN** identical moved closures do not appear as added and removed, changed anchored callbacks appear once, and unsafe cases remain visibly ambiguous

#### Scenario: Marketing scopes are reviewed
- **WHEN** root, Astro file, Astro directory, and diff commands run
- **THEN** every scope remains exact and Astro appears as unsupported selected source in terminal and JSON

#### Scenario: Netdisco is reviewed
- **WHEN** root and recent diff commands run
- **THEN** bundled assets do not own default output and authored Rust and JavaScript remain visible

#### Scenario: Swiftide and Parity are reviewed
- **WHEN** their recorded diffs run
- **THEN** Swiftide keeps precise method findings and Parity's documentation-only change says `No debt changed.`
