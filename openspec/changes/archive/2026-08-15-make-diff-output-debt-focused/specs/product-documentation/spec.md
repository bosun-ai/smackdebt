## MODIFIED Requirements

### Requirement: README demonstrates progressive diff exploration
The README SHALL show a default ref comparison that answers which human debt
worsened, improved, or changed, identifies a debt-bearing area, and drills into
that scope. It SHALL show exact no-debt behavior, source-only verdict counts and
navigation, independent architecture/history sections, the exact unique
changed-file warning, middle-dot anonymous identities with next-line
`path:line`, meaningful measurements and locations, and no raw/context rows.

#### Scenario: Reader follows a diff example
- **WHEN** they run the documented default and path commands
- **THEN** the example shows debt direction, useful evidence, and scope-only drill behavior

### Requirement: README states JSON detail retention
The README SHALL state that terminal default and `--all` diff views contain
human debt movement only, while JSON version 3 retains every complete
comparison, direction, count, architecture relation change, history context,
diagnostic, and machine identity. It SHALL state that private selections and
`DebtDiffCounts` are not JSON fields.

#### Scenario: Integration needs unchanged or healthy context
- **WHEN** terminal omits that comparison
- **THEN** documentation directs the integration to complete JSON version 3

### Requirement: README documents terminal presentation controls
The README and CLI help SHALL define `--all` in a diff as every trusted human
primary/test/example/benchmark debt row, not all retained comparisons. They
SHALL state that fixture, generated, and recovered comparisons remain JSON-only
in diff default, diff `--all`, and diff path because their roles/matching are
excluded from trusted diff verdicts, while selected recovered facts produce one
grouped real warning. They SHALL state that this narrows only diff human detail
and codebase behavior remains unchanged. They SHALL state that a
selected path changes scope only and JSON remains complete. Examples SHALL cover
default and `--all` at useful widths without promising a line budget for
`--all`.

#### Scenario: User asks for all diff detail
- **WHEN** they read `--all` help
- **THEN** they expect every eligible trusted debt row, no fixture/generated/recovered comparison row, and no unchanged, ambiguous-card, healthy-only, raw relation, or history-context row

### Requirement: Documented command examples are checked
README and help examples SHALL be exercised as exact black-box CLI tests.
Audits SHALL cover source-only, architecture-only, history-only, and mixed
separation; stable direction/rank; source-only counts/navigation; no-debt
sentences; exact `<Warning glyph> 1 changed file could not be checked.` and
`<Warning glyph> <count> changed files could not be checked.` rows; recovered-only
and fixture/generated-only default, `--all`, and path; unchanged role, coverage,
measurement, comparison, diagnostic, and recovered JSON; every required measurement;
locations; middle-dot anonymous identities and next-line `path:line`;
section absence, path scope, `--all`, complete JSON, simple wording, and absence
of angle-bracket identity placeholders.

#### Scenario: Human documentation changes
- **WHEN** acceptance runs the documented diff commands
- **THEN** stdout, stderr, status, and width-safe bytes match reviewed examples exactly
