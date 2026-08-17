## 1. Rust cfg(test) scope

- [x] 1.1 Add languages unit tests for the matching rule covering all six decided forms: `#[cfg(test)]`, `#[cfg(all(test, not(loom)))]`, and `#[cfg(any(test, fuzzing))]` match; `#[cfg(not(test))]`, `#[cfg(feature = "test")]`, and `#[cfg_attr(test, ...)]` do not.
- [x] 1.2 Add a languages test proving an ancestor `mod` attribute scopes references declared in nested items.
- [x] 1.3 Add the `DependencyScope { Default, Test }` field to `DependencySyntax` and detect test scope from outer `cfg` attributes on the item and its ancestor modules.
- [x] 1.4 Prove `offset_dependency` preserves the scope when it rebuilds the syntax record.
- [x] 1.5 Add a project test proving a primary file with a `#[cfg(test)]` module yields a test-role relation, and a separate primary relation when the same target is also imported outside the test scope.
- [x] 1.6 Apply `role = max(file role, test)` for test-scoped references in every record path, and prove a fixture or generated file keeps its own role.
- [x] 1.7 Add `#[cfg(test)]` content to a generated fixture so committed JSON pins `"role":"test"`, and confirm no other committed snapshot moves.
- [x] 1.8 Verify on the field repositories that tokio's JSON shows test-role relations from `src/**` `#[cfg(test)]` modules while scikit-learn and opencode stay byte-identical.

## 2. Primary-only verdict graphs

- [x] 2.1 Add `DependencyEdge::enters_verdict_graph()` as `uses && trusted && primary`, keeping `affects_verdict()` unchanged for evidence eligibility.
- [x] 2.2 Build package dependency edges, the package cycle graph, the file cycle graph, the package graph's fan-in, fan-out, and instability, and the stable-dependency comparison from `enters_verdict_graph()` only.
- [x] 2.3 Keep coverage partitioning on `affects_verdict()` and prove coverage counts, external dependency counts, and resolution diagnostics are unchanged.
- [x] 2.4 Thread `explanation_pairs` — file-edge pairs satisfying `affects_verdict()` that cross packages, unioned with manifest-name explanation pairs — through the architecture build into coupling explanation and comparison.
- [x] 2.5 Supply `explanation_pairs` at both call sites including the diff before side, and add a diff expectation proving no spurious Worse finding row appears for a dev-dependency repository.
- [x] 2.6 Reword orphan fan-in to trusted eligible uses and add a test proving a primary file imported only by tests is not an orphan.
- [x] 2.7 Add a fixture where a test-role relation would close a package cycle: no finding is created, the relations appear in JSON, and the pair's coupling stays explained.
- [x] 2.8 Add a primary stable-dependency fixture that authors the product's first committed terminal and JSON evidence for an SDP finding.
- [x] 2.9 Audit consumers that assume `(source, target)` uniqueness in `dependency_edges`, starting with the architecture comparison.
- [x] 2.10 Verify on the field repositories that tokio reports no package cycle and no stable-dependency finding while the other four repositories' package graphs and orphan lists are unchanged.

## 3. Test-declared module files

- [x] 3.1 Add project tests for a `#[cfg(test)] mod name;` declaration: the declared file is test source, its relations leave the verdict graphs, a file also declared outside a test scope stays primary, a transitively declared file is test source, and an explicitly configured role wins.
- [x] 3.2 Read a Rust relative candidate against the module directory the declaring file owns, then resolve module declarations to files and classify a file as test when it has at least one declaration and every declaration is test-scoped, by a deterministic fixpoint over ordered structures.
- [x] 3.3 Apply the classification before findings, ratings, coverage, and history evidence read a role, on the codebase path and on both sides of a diff.
- [x] 3.4 Verify on the field repositories that tokio reports no package dependency cycle and that scikit-learn, opencode, kwaak, and fluyt keep their verdicts.

## 4. Module-wiring cycle exclusion

- [x] 4.1 Extend `rust_module_ownership_cycle_is_context_while_mutual_uses_are_a_verdict` with a suppressed parent-child pair, a surviving sibling cycle, and a surviving mutual-uses pair between unowned files.
- [x] 4.2 Exclude, from the file cycle graph only, `uses` relations between a file pair that also carries a `module_ownership` relation in either direction.
- [x] 4.3 Apply the identical exclusion in the cycle witness lookup and prove a suppressed pair produces neither a cycle nor a witness.
- [x] 4.4 Prove the exclusion is pairwise by keeping a cycle that passes through an owning pair via other files.
- [x] 4.5 Prove fan-in, fan-out, instability, and orphan facts are unaffected by the exclusion.
- [x] 4.6 Add a fixture that pins the suppression in committed terminal and JSON bytes.
- [x] 4.7 Verify on the field repositories that tokio's `fs/` and `io/` cycles are gone while scikit-learn's `_config`-`_array_api`, opencode's three cycles, and smackdebt's `change_coupling`-`evolution` remain.

## 5. Documentation

- [ ] 5.1 Explain in the README that verdict graphs use primary-role relations only and that test, example, and benchmark relations stay complete as context.
- [ ] 5.2 Explain in the README that a Rust reference under a `#[cfg(test)]` scope carries the test role by a syntactic rule, and that `no code dependency` still accounts for those relations.
- [ ] 5.3 Explain in the README that imports between a Rust module-owning file pair are excluded from the file cycle graph, and that the exclusion is limited to that pair.
- [ ] 5.4 Update `ARCHITECTURE.md` for the predicate split, the scope field, and the cycle-graph exclusion.
- [ ] 5.5 Grep the README and `ARCHITECTURE.md` for stale `trusted eligible uses` wording that now means the verdict graph.

## 6. Evidence and close

- [ ] 6.1 Review every newly authored fixture and golden file as a product artifact rather than as regenerated output.
- [ ] 6.2 Confirm ratings, signals, unit measurements, exit codes, work counts, and the JSON schema shape are unchanged, and that no report-schema delta is required.
- [ ] 6.3 Run the five-repository matrix and record that every previously validated true positive survives.
- [ ] 6.4 Spot-check serial and parallel runs for byte-identical terminal and JSON output on tokio.
- [ ] 6.5 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [ ] 6.6 Archive this change.
