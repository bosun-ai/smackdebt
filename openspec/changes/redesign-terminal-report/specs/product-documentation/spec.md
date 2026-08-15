## MODIFIED Requirements

### Requirement: README documents terminal presentation controls
Product documentation SHALL explain that severity, direction, and diagnostics are stated in
words, that glyphs and the tier bar are optional decoration resolved from
terminal detection, and that piped output is the same report in words with no
private-use codepoint. It SHALL explain `--color`, `NO_COLOR`, width behavior,
content-aware row stacking, `--all` as all useful debt, and JSON as the complete
machine-readable view of everything the terminal omits. It SHALL state that
there is no icon option, emoji mode, or theme setting.

The previously documented commitment that there is no ASCII fallback SHALL be
removed, because undecorated words-only output is now the defined behavior for
non-terminal and color-disabled output.

#### Scenario: User runs Smackdebt in automation
- **WHEN** the user reads output guidance
- **THEN** the README explains word-only piped output, how to force or disable decoration and ANSI styling, and that JSON remains unstyled and complete

#### Scenario: User looks for the fallback statement
- **WHEN** the user reads presentation documentation
- **THEN** no claim that ASCII or plain fallback is unavailable appears

### Requirement: Documentation covers the unified result
The README SHALL show how one command answers code and architecture questions, opening
with the verdict block — scope, tier sentence, word-labeled counts, and worst
offender — followed by `AREAS`, `FINDINGS`, `ARCHITECTURE`, `HISTORY`, and
`WARNINGS` sections, with empty optional sections omitted. It SHALL list the
frozen codebase and diff tier ids with their sentences, state that a clean diff
prints the verdict line only, and state that every count is labeled with its
word including zero counts.

#### Scenario: A user reads the main examples
- **WHEN** the user follows documented default, path, and diff examples
- **THEN** the examples lead with the verdict, label every count, omit empty sections, and do not present a combined score

#### Scenario: A machine consumer reads the documentation
- **WHEN** an integration author decides what to key on
- **THEN** the README names the tier ids as the stable vocabulary and points to JSON for complete data
