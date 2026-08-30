## 1. Single-line dependency specifiers

- [x] 1.1 Reduce an extracted dependency target to its first line with collapsed whitespace and no shortening, wherever a language falls back to the declaration node it read.
- [x] 1.2 Add a language fixture with a three-line template-literal dynamic import and assert that no retained resolution diagnostic target contains a newline, carriage return, or tab.
- [x] 1.3 Regenerate any snapshot the fix moves, per case.

## 2. Dependency edges leave the human terminal

- [ ] 2.1 Delete the relationship rows and the scope-derived gate that switched them on, and reduce the remaining detail gate to `--all`.
- [ ] 2.2 Gate the unresolved and ambiguous rows on `--all` or a file scope, keeping the grouped warning sentence at every scope.
- [ ] 2.3 Invert the acceptance tests that asserted retained incoming edges, current diff edges, and external unresolved rows above a file scope, and drop the import-count fact from the long-fact family case.
- [ ] 2.4 Promote the absence of an import-count fact, an ownership row, and a path-to-path arrow outside a cycle witness to an invariant over every committed terminal snapshot.
- [ ] 2.5 Regenerate the affected terminal snapshots one by one and record the expected diff shape in the commit body.

## 3. The problem card model and its detectors

- [ ] 3.1 Add the problem module in analysis: pattern, anchor, evidence, card, policy constants, and a clustering entry point over borrowed slices.
- [ ] 3.2 Add the size-finding identity type so a card can link a size finding by index.
- [ ] 3.3 Implement the detectors in claiming order with integer-only rules, file degree over verdict-graph edges, and the package-relative nearest-rank median computed once per package.
- [ ] 3.4 Implement the descriptive-card rule so a widely imported file with no rated or size finding carries no rating and stays out of default detail.
- [ ] 3.5 Implement the problem rank and sort the table once when the report is finished, without recording an algorithm pass.
- [ ] 3.6 Add pure boundary tests per detector — two High findings producing no god file, fan-in 7 against 8, exactly four times the median — plus a claimed-once audit, an all-keys-tie rank test, and a serial-parallel equality test.
- [ ] 3.7 Regenerate the analysis API snapshot for the new surface.

## 4. Problem cards in the machine report

- [ ] 4.1 Serialize the `problems` table in rank order with pattern, rating, visibility string, anchor, ordered evidence, and claimed findings, and extend the checked schema in the same commit.
- [ ] 4.2 Serialize the size-finding position contract so evidence indexes resolve, and assert index integrity and claimed-once in JSON validation.
- [ ] 4.3 Prove no problem value is a floating-point number or a boolean.
- [ ] 4.4 Regenerate the JSON snapshots and review them through the JSON textconv.

## 5. The problem section and the one-screen budget

- [ ] 5.1 Replace the codebase finding, architecture, and history sections with one problem section built from the ranked table filtered by anchor, keeping the diff sections unchanged.
- [ ] 5.2 Add the exact human name per pattern and the exact wording per evidence kind, with correct singular and plural form, reusing the existing row and stacked-witness machinery.
- [ ] 5.3 Implement the slot budget and its ladder, `--top N` as cards selecting the rung `N` selects, `--all` as every card with complete evidence including descriptive ones, and a file scope as complete evidence without the ladder.
- [ ] 5.4 Add evidence that a directory-scope default fits the budget, that the view the `next:` line proposes fits it, that `--top 10` shows ten cards, and that one invocation states the same cards at 50 and at 120 columns.
- [ ] 5.5 Add generated fixtures and exact acceptance for each frozen pattern, including one file with three High findings producing one card and one card per strongly connected component.
- [ ] 5.6 Regenerate every affected terminal snapshot case by case, never as a batch, keeping the fifty-column display-width audit passing.

## 6. The repository frame

- [ ] 6.1 Add the analysis-owned repository-share fact with its frozen sentence, absent at the repository root and when the repository holds no High debt, proven never to move the tier.
- [ ] 6.2 Render the share row inside the sub-scope verdict block from the analysis-owned bytes.
- [ ] 6.3 Serialize the share beside the tier and sentence, extend the schema, and add exact acceptance at package and directory scope and for its absence at the root.

## 7. Calibration, documentation, and close-out

- [ ] 7.1 Calibrate the thresholds against real repositories: run the built binary over a Vue application and over this workspace at repository, package, directory, and file scope, review the resulting card sets, and pay particular attention to the rated-unit arm of the `god_file` rule where single-file components inflate unit counts.
- [ ] 7.2 Freeze the reviewed constants, updating the specs where a proposed value moved.
- [ ] 7.3 Update the README: the problem section in the codebase example, the pattern list with its words and thresholds, the one-screen budget, the new meaning of `--top` and `--all`, the share sentence, and the statement that edges are JSON-only.
- [ ] 7.4 Update `ARCHITECTURE.md` for the clustering step and the analysis-owned card order against the renderer's per-variant words.
- [ ] 7.5 Confirm the committed ratchet baseline is unchanged with the gate, note the stale performance `report_digest` values, tick every task, pass strict validation and the complete check, and archive this change.
