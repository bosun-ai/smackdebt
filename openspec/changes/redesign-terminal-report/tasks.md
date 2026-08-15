## 1. Audience contract

- [x] 1.1 Introduce the word vocabulary `high`, `watch`, `worse`, `better`, `changed`, `warning`, and `next:` across every human row.
- [x] 1.2 Resolve a decoration setting beside color from the existing terminal detection, with no new CLI option.
- [x] 1.3 Make glyphs and the tier-colored bar appear only with decorations and always adjacent to the word they decorate.
- [x] 1.4 Make `--color never` fully plain and assert that piped output contains no codepoint in U+E000–U+F8FF for every public flow.
- [x] 1.5 Prove decorated and undecorated output carry the same facts and that stripping ANSI from decorated output leaves the same words.

## 2. Verdict block

- [x] 2.1 Replace the header, quality, and change lines with a verdict block showing scope, tier sentence, labeled counts, and the worst offender with its resolved path and reason.
- [x] 2.2 Print the tier sentence bytes owned by analysis without composing new wording in the renderer.
- [x] 2.3 Label every diff count with its word, name the family that moved, and print zero counts rather than omitting them.
- [x] 2.4 Print the verdict line only for a clean diff, with no history, warnings, or trailing sections.
- [x] 2.5 Add snapshots for every codebase tier and every diff tier.

## 3. Sections and diff cards

- [x] 3.1 Show at most five debt-bearing children under `AREAS` with word-labeled counts.
- [x] 3.2 Add `· hot (n commits)` to findings whose file is a hotspot and keep `path:line` and measurements on every source finding.
- [x] 3.3 Stack cycle witnesses across lines and never truncate a witness with an ellipsis.
- [x] 3.4 Add stable-dependency rows to `ARCHITECTURE` using integer operands.
- [x] 3.5 Add knowledge-concentration rows to `HISTORY` as counts and show one row per package pair.
- [x] 3.6 Group `WARNINGS` into one sentence per kind, such as `29 imports could not be followed.`
- [x] 3.7 Write the discover line as `next: smackdebt <path>`.
- [x] 3.8 Give every diff finding `path:line`, before and after values for each changed measurement, and a human identity for anonymous units so no `<closure 1177>` identity is printed.

## 4. Detail and width

- [x] 4.1 Redefine `--all` as all useful debt and permanently remove raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, and healthy rows from terminal output.
- [x] 4.2 State in documentation that JSON remains the complete view of everything removed.
- [x] 4.3 Replace report-level width tiers with per-row content-aware aligned-or-stacked writing.
- [x] 4.4 Prove no measurement, count, witness, evidence value, command, or identity is silently clipped at widths 120, 100, 80, and 50.

## 5. Errors

- [x] 5.1 Emit exactly `smackdebt: path not found: <user-path>\n` with status 1 and empty stdout, using the original argument without an absolute path or operating-system text.
- [x] 5.2 Emit exactly `smackdebt: Git ref not found: <ref>\n` with status 1 and empty stdout, without Git command, status, or fatal output.
- [x] 5.3 Emit exactly `smackdebt: --all cannot be used with --json\n` with status 2 and empty stdout.
- [x] 5.4 Append no usage or help to any of the three failures.

## 6. Documentation and evidence

- [x] 6.1 Rewrite the README examples, vocabulary, and terminal guidance for the new report.
- [x] 6.2 Remove the "no ASCII fallback" commitment and document decoration behavior in its place.
- [x] 6.3 Update `ARCHITECTURE.md` for decoration resolution, per-row content-aware writing, and the unchanged renderer-does-no-analysis boundary.
- [x] 6.4 Add piped-versus-tty byte evidence and the private-use codepoint assertion to acceptance.
- [x] 6.5 Add exact error-byte acceptance for all three failures.
- [x] 6.6 Add the end-to-end contradiction case where a diff introduces a package cycle and the verdict reads worse.
- [x] 6.7 Regenerate every terminal snapshot and review each file individually.
- [x] 6.8 Prove report facts, rank, analysis, exit classes, stream placement, work counts, and JSON version 3 bytes are unchanged.
- [x] 6.9 Re-run the release binary on smackdebt, swiftide, and fluyt and record that swiftide `diff HEAD~15` leads with a one-line verdict instead of 204 changed rows.
- [x] 6.10 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [ ] 6.11 Archive this change before implementing `adopt-report-schema-v4`.
