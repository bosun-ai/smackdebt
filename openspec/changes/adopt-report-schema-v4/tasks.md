## 1. Version 4 contract

- [ ] 1.1 Set `schema_version` to 4 and stop emitting any version-3 or version-2 compatibility object.
- [ ] 1.2 Add the checked `report-v4` JSON Schema and retire the version-3 schema file.
- [ ] 1.3 Stream the denormalized head first: `verdict` with `tier`, `sentence`, and `mode`.
- [ ] 1.4 Stream `summary` with checked, high, and watch counts, word-labeled debt-diff counts, and up to three fully resolved worst entries carrying path strings, identity, and reason.
- [ ] 1.5 Assert in acceptance that head values agree with the tables they duplicate.

## 2. New tables and fields

- [ ] 2.1 Add the `hotspots` table with file index, maximum unit rating, and touch count.
- [ ] 2.2 Add the `size_findings` table with scope index, file or container kind, measured value, and triggered threshold.
- [ ] 2.3 Add the `orphan_files` table as a descriptive file-index table.
- [ ] 2.4 Add the `kind` discriminator to architecture findings including `stable_dependency_violation` with both packages' integer degree operands and the reference count.
- [ ] 2.5 Add the `kind` discriminator to evolutionary findings for unexplained coupling and knowledge concentration.
- [ ] 2.6 Add the history window length in days and the window-excluded commit count to `history_coverage`.
- [ ] 2.7 Add `manifest_name` to package records, absent when no manifest declares one.

## 3. Integer-only serialization

- [ ] 3.1 Remove serialized `similarity` and `ratio` floating-point values while keeping their integer operands.
- [ ] 3.2 Assert that no serialized value is a floating-point number in any acceptance JSON result.
- [ ] 3.3 Keep coupling similarity and concentration ratio as derived values used for threshold evaluation and human presentation only, and prove no documentation or spec text still promises them in the machine report.

## 4. Rating promotion

- [ ] 4.1 Promote maximum nesting depth to a rated signal at Watch 4 and High 7.
- [ ] 4.2 Promote parameter count to a rated signal at Watch 6 and High 9.
- [ ] 4.3 Add both thresholds to health policy and to configuration alongside the existing signals.
- [ ] 4.4 Add pure boundary tests at 3 and 4, 6 and 7, 5 and 6, and 8 and 9.
- [ ] 4.5 Re-derive comparisons with the new signals and review every flipped rating.

## 5. Documentation and retirement

- [ ] 5.1 Update `schemas/README.md` for version 4 and remove the version-3 schema reference.
- [ ] 5.2 Update `ARCHITECTURE.md` version-3 references.
- [ ] 5.3 Amend the `AGENTS.md` three-rated-measurements rule to five rated measurements.
- [ ] 5.4 Rewrite the README JSON section for version 4, its head, and the removal of serialized similarity and ratio floats, ensuring no documented text still promises those floats.
- [ ] 5.5 Remove the `report-schema-v3` and `report-schema-v2` capabilities from accepted specs through this change's archival.

## 6. Evidence and gates

- [ ] 6.1 Validate codebase, clean ref-diff, and mixed worktree-diff results against the version-4 schema with semantic, index, privacy, and exact-byte checks from the same invocation.
- [ ] 6.2 Add evidence for every new table, discriminator, window field, and `manifest_name`.
- [ ] 6.3 Prove the head answers verdict and summary without joining tables.
- [ ] 6.4 Prove serial and parallel JSON bytes stay identical and terminal sections and vocabulary are unchanged.
- [ ] 6.5 Regenerate every JSON snapshot and review each file individually.
- [ ] 6.6 Re-run the release binary on smackdebt, swiftide, and fluyt and read the head of each JSON result as the product review.
- [ ] 6.7 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [ ] 6.8 Confirm `prepare-first-release` may resume only after this change is archived.
