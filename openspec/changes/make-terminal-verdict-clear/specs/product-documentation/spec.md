## MODIFIED Requirements

### Requirement: README demonstrates debt distribution
The README SHALL show the exact two-line non-empty `QUALITY` verdict with a
grouped checked count such as `1,686 checked`, integer-safe nearest-tenth
attention percent, and exact High and Watch counts. It SHALL distinguish
checked greater than zero with zero attention, whose second line is `No
findings.`, from checked zero, whose sole line is `Nothing was checked.` Its
codebase area example SHALL use exact High and Watch counts in stable
severity-led order without debt share, local
rate, percentage, or bar display. The example SHALL retain leading findings and
the next drill command.

#### Scenario: New user follows progressive exploration
- **WHEN** the user reads the codebase example
- **THEN** the example distinguishes empty, zero-attention, and non-zero quality verdicts and moves from repository to child area to file using exact, internally consistent counts

### Requirement: README demonstrates progressive diff exploration
The README SHALL show a responsive diff summary whose area rows retain exact
Worse, Better, and Changed counts in stable order without change-share
percentages or bars. It SHALL show how to select an affected area and reach its
meaningful unit changes without defining the later debt-focused diff redesign.

#### Scenario: User locates a worktree regression
- **WHEN** the user reads the diff example
- **THEN** exact direction counts identify the affected area and the drill command reaches its concise comparison

### Requirement: README explains share and completeness
The README SHALL state that report and JSON facts retain codebase debt share,
local attention rate, diff change share, exact denominators, and complete
results while human area rows intentionally show exact status counts instead of
those ratios or bars. It SHALL explain path-limited scope, default terminal
limits, `--all`, and complete JSON behavior.

#### Scenario: User compares terminal and JSON
- **WHEN** a terminal area row has no percentage or bar
- **THEN** the README explains that exact counts drive the human drill decision and complete distribution facts remain in JSON

#### Scenario: User needs every retained result
- **WHEN** the default terminal view omits rows or details
- **THEN** the README explains terminal `--all` and that JSON is always complete

### Requirement: Product examples explain share and rate
Product documentation SHALL distinguish the report's debt share, local
attention rate, and diff change share from the terminal verdict. It SHALL state
that terminal area rows do not render these rates, shares, percentages, or bars
and that JSON version 3 retains the exact facts.

#### Scenario: User reads the root example
- **WHEN** the README introduces the codebase report
- **THEN** it explains that the overall attention percent summarizes checked source while exact area status counts choose the next drill target

### Requirement: README documents terminal presentation controls
Product documentation SHALL explain the Nerd Font glyph requirement, `--color`,
`NO_COLOR`, content-aware width behavior, Unicode redirected output, `--all`,
and JSON as the complete machine-readable view. It SHALL state that rows use a
compact form when content fits and otherwise stack or middle-shorten long
identities without silently losing facts. It SHALL state that there is no icon
option, emoji mode, or ASCII fallback.

#### Scenario: User runs Smackdebt in automation
- **WHEN** the user reads output guidance
- **THEN** the README explains ANSI control, the 100-column redirected default, content-aware stacking, and unstyled complete JSON

### Requirement: Product documentation states privacy and coverage behavior
The README SHALL state that analysis stays local, contributor identities are
not reported, and unavailable or incomplete history is shown explicitly. It
SHALL distinguish selected-source read failure, failed parse, and recovered
advisory parse from fixture/generated policy exclusions. It SHALL state that
primary, test, example, and benchmark source affect the default verdict. It
SHALL show the separate exact Warning-glyph `<count> import(s) could not be
followed.` row for unmatched or ambiguous imports without exposing
implementation diagnostics.

#### Scenario: A user sees a coverage warning
- **WHEN** the user consults coverage documentation
- **THEN** real incomplete analysis is distinguished from fixture/generated policy and from the separate exact architecture warning

### Requirement: Documented command examples are checked
The README SHALL mark runnable console examples and associate each with a named
public generated fixture, expected status, and exact relevant output fragments.
Checked examples SHALL include grouped checked digits, integer-safe halfway-up
rounding, empty, zero-attention, and non-zero verdicts, ordered areas without
rates or shares, required grouped and named real gaps, the exact architecture
warning, content-aware width behavior, and byte-exact missing-path, missing-ref,
and `--all --json` status/stdout/stderr results without usage or help tails.
The three checked stderr values SHALL be exactly `smackdebt: path not found:
<user-path>\n`, `smackdebt: Git ref not found: <ref>\n`, and `smackdebt: --all
cannot be used with --json\n`, with statuses 1, 1, and 2 respectively and empty
stdout.

#### Scenario: Documentation tests run
- **WHEN** documentation validation reads runnable README examples
- **THEN** it executes them through the built CLI and matches status, sections, glyphs, commands, stdout, and stderr in order

### Requirement: Documentation covers the unified result
The README SHALL show how one command answers code and architecture questions
through separate `QUALITY`, `FINDINGS`, `ARCHITECTURE`, and `HISTORY` sections,
with empty optional sections omitted. `QUALITY` SHALL lead with the two-line
checked-source verdict or sole empty-selection line. Source coverage and the
exact architecture Warning-glyph row SHALL be separate and SHALL appear only
when their accepted conditions exist.

#### Scenario: A user reads the main examples
- **WHEN** the user follows documented default, path, and diff examples
- **THEN** examples lead with the clear verdict, retain relevant findings, omit empty optional sections and area ratios, and do not present a combined score
