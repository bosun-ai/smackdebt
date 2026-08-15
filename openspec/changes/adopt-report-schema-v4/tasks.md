## 1. Version 4 contract

- [x] 1.1 Set `schema_version` to 4 and stop emitting any version-3 or version-2 compatibility object.
- [x] 1.2 Add the checked `report-v4` JSON Schema and retire the version-3 schema file.
- [x] 1.3 Stream the denormalized head first: `verdict` with `tier`, `sentence`, and `mode`.
- [x] 1.4 Stream `summary` with checked, high, and watch counts, word-labeled debt-diff counts, and up to three fully resolved worst entries carrying path strings, identity, and reason.
- [x] 1.5 Assert in acceptance that head values agree with the tables they duplicate.

## 2. New tables and fields

- [x] 2.1 Add the `hotspots` table with file index, maximum unit rating, and touch count.
- [x] 2.2 Add the `size_findings` table with file index, file or container subject, container name, measured value, and triggered rating.
- [x] 2.3 Add the `orphan_files` table as a descriptive file-index table.
- [x] 2.4 Add the `stable_dependency_findings` table with kind `stable_dependency_violation`, both packages' integer degree operands, the reference count, and witness edges.
- [x] 2.5 Add the `knowledge_concentration_findings` table with kind `knowledge_concentration` and counts without identity, and state each finding family's `kind` on its own table.
- [x] 2.6 Add the history window length in days and the window-excluded commit count to `history_coverage`.
- [x] 2.7 Add `manifest_name` to package records, absent when no manifest declares one.
- [x] 2.8 Serialize each comparison's nullable source location so JSON states the `path:line` the terminal prints.

## 3. Integer-only serialization

- [x] 3.1 Remove serialized `similarity` and `ratio` floating-point values while keeping their integer operands.
- [x] 3.2 Assert that no serialized value is a floating-point number in any acceptance JSON result.
- [x] 3.3 Keep coupling similarity and concentration ratio as derived values used for threshold evaluation and human presentation only, and prove no documentation or spec text still promises them in the machine report.

## 4. Rating promotion

- [x] 4.1 Promote maximum nesting depth to a rated signal at Watch 4 and High 7.
- [x] 4.2 Promote parameter count to a rated signal at Watch 6 and High 9.
- [x] 4.3 Add both thresholds to health policy and to configuration alongside the existing signals.
- [x] 4.4 Add pure boundary tests at 3 and 4, 6 and 7, 5 and 6, and 8 and 9.
- [x] 4.5 Re-derive comparisons with the new signals and review every flipped rating.

## 5. Documentation and retirement

- [x] 5.1 Update `schemas/README.md` for version 4 and remove the version-3 schema reference.
- [x] 5.2 Update `ARCHITECTURE.md` version-3 references.
- [x] 5.3 Amend the `AGENTS.md` three-rated-measurements rule to five rated measurements.
- [x] 5.4 Rewrite the README JSON section for version 4, its head, and the removal of serialized similarity and ratio floats, ensuring no documented text still promises those floats.
- [ ] 5.5 Remove the `report-schema-v3` and `report-schema-v2` capabilities from accepted specs through this change's archival.

## 6. Evidence and gates

- [x] 6.1 Validate codebase, clean ref-diff, and mixed worktree-diff results against the version-4 schema with semantic, index, privacy, and exact-byte checks from the same invocation.
- [x] 6.2 Add evidence for every new table, discriminator, window field, and `manifest_name`.
- [x] 6.3 Prove the head answers verdict and summary without joining tables.
- [x] 6.4 Prove serial and parallel JSON bytes stay identical and terminal sections and vocabulary are unchanged.
- [x] 6.5 Regenerate every JSON snapshot and review each file individually.
- [x] 6.6 Re-run the release binary on smackdebt, swiftide, and fluyt and read the head of each JSON result as the product review.
- [x] 6.7 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [x] 6.8 Confirm `prepare-first-release` may resume only after this change is archived.
