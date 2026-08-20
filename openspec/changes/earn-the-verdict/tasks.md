## 1. Repo hygiene

- [x] 1.1 Add the MIT `LICENSE` matching the workspace manifest's declared license.
- [x] 1.2 Delete the dead evidence-map script and its documentation and drop its line from the architecture recipe.
- [x] 1.3 Make JSON snapshots reviewable: `diff=json` attributes for snapshot and schema JSON, the documented textconv configuration, and a `show-snapshot` recipe.

## 2. Continuous integration

- [ ] 2.1 Add one workflow that runs the complete `just check` on every push and pull request, on Linux and macOS, from a full-depth checkout, installing only tools `just check` already needs.

## 3. Hermetic acceptance environment

- [ ] 3.1 Pin `HOME`, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL`, and `GIT_CONFIG_SYSTEM` for acceptance child processes and fixture git commands through one shared helper.
- [ ] 3.2 Prove the pinning moves zero snapshot bytes.

## 4. Discovery on git ignore semantics

- [ ] 4.1 Rewrite the walk on the `ignore` crate with the pinned serial builder configuration, keeping the public discovery types, per-directory name order, and the final repository-relative sort.
- [ ] 4.2 Layer configuration excludes as gitignore patterns anchored at the analyzed root, with working anchors and negations.
- [ ] 4.3 Add unit tests for nested `.gitignore` override, `!` re-include, root-anchored patterns, `.git/info/exclude`, configuration anchors and negations, dependency-directory precedence over negation, and identical order across two walks.
- [ ] 4.4 Add global-gitignore acceptance evidence through the pinned configuration home, the subpath ancestor-ignore guard test, and hand-updated visited-entry evidence with an explanation per delta.
- [ ] 4.5 Run the license check over the new dependency tree and prove zero terminal and JSON byte changes.

## 5. Nested checkout exclusion

- [ ] 5.1 Prune every directory below the analyzed root containing a `.git` entry, directory or file, and never the analyzed root itself.
- [ ] 5.2 Record pruned checkouts as `nested_repository` diagnostics in the report, JSON, and schema.
- [ ] 5.3 Emit the default-output disclosure sentence with a per-kind subject and add fixtures for both `.git` shapes.

## 6. Grouped import resolution

- [ ] 6.1 Emit one Rust reference per imported item of a grouped `use` list, reconstructing paths from enclosing list prefixes, without changing the shared language dependency contract.
- [ ] 6.2 Add fixtures for flat lists, nested lists, re-exported lists, `as` clauses, `self` members, and test-scoped lists with per-item role demotion.
- [ ] 6.3 Regenerate churned snapshots per case and update documented import-count literals.

## 7. Windowed history streaming

- [ ] 7.1 Filter streamed history inside the git process on landed (committer) dates with an epoch cutoff, keeping the in-process boundary filter.
- [ ] 7.2 Redefine stream coverage counts over the windowed set with the window-excluded counter counting boundary rejects.
- [ ] 7.3 Add adapter, pure, and acceptance evidence and regenerate the git and project API snapshots for the signature change.

## 8. Empty-window disclosure

- [ ] 8.1 Emit the empty-window warning sentence when a complete stream yields zero commits inside a selected window.
- [ ] 8.2 Add an acceptance case with all commits older than the window proving the sentence and surviving source and architecture results.

## 9. Transitive-aware coupling wording

- [ ] 9.1 Compute a per-pair link classification — direct, indirect via a named first intermediate, or none — once at report build, checked in both directions.
- [ ] 9.2 Render indirect pairs as `no direct dependency` with `linked via <package>` while keeping direct and unreachable wording and finding creation unchanged.
- [ ] 9.3 Add pure three-package-chain tests for all three variants, an acceptance chain case, and move every documented literal together.

## 10. Unfollowed-import breakdown

- [ ] 10.1 Break the unfollowed-imports warning into its causes, printing each fact only when non-zero.
- [ ] 10.2 Add acceptance evidence with both causes in one scope and with a single cause, moving documented literals and snapshots together.

## 11. Volume terms in the codebase tier

- [ ] 11.1 Add the small-scope cap and absolute volume floors as integer-only terms combined by the existing max-floor rule, with all four constants flagged for review.
- [ ] 11.2 Add pure boundary tests at each constant and one unit either side, keeping the accepted permille boundary scenarios unchanged.
- [ ] 11.3 Regenerate per-scope verdict snapshots per case and add exact black-box boundary evidence.

## 12. Unsupported-share qualifier

- [ ] 12.1 Record file sizes from walk metadata and expose selected and unsupported byte totals in coverage, JSON, and the schema.
- [ ] 12.2 Add the analysis-owned qualifier with its frozen sentence and share fact, proven never to move the tier.
- [ ] 12.3 Render the qualifier row under the verdict and beside the tier in JSON, regenerating snapshots and API snapshots.

## 13. `--top`

- [ ] 13.1 Add `--top N` limiting displayed findings only, rejecting zero and conflicting with `--json` and `--all`.
- [ ] 13.2 Add tests for a smaller and larger limit, the zero rejection, and both conflicts, updating the help golden and README.

## 14. The gate command

- [ ] 14.1 Add pure gate policy in analysis: frozen time-invariant signal ids, verdict-affecting snapshot rows, and exact comparison rules.
- [ ] 14.2 Add strict baseline read and write in the CLI, rejecting unknown signals, duplicates, and out-of-order rows.
- [ ] 14.3 Add the `gate` subcommand with exit code 3 on regression and the exact missing-baseline failure, rendering the gate report in house vocabulary.
- [ ] 14.4 Add pure comparison tests and acceptance evidence for clean, regressed, and missing-baseline runs, updating the help golden and the README exit-code table and tombstone.

## 15. Self-gating

- [ ] 15.1 Add `--update` writing the observed snapshot verbatim, byte-stable and idempotent.
- [ ] 15.2 Add gate `--json` with its checked schema and validated acceptance evidence.
- [ ] 15.3 Commit smackdebt's own baseline and wire the gate into `just check` last, documenting the loop.

## 16. Close the change

- [ ] 16.1 Tick every task, pass strict validation and the complete check including the gate, confirm `prepare-first-release` stays coherent, and archive this change.
