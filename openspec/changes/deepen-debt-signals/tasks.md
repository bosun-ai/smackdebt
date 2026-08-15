## 1. History window governs history signals

- [x] 1.1 Apply the `--history` cutoff when history records become facts so churn, touches, coupling, and concentration all describe the same window.
- [x] 1.2 Expose the window length in days and the count of streamed commits excluded by the window in history coverage, counted separately from other exclusions.
- [x] 1.3 Add pure tests proving a windowed run and an unwindowed run differ in churn, coupling, and concentration and that coverage states the window.

## 2. Hotspots and rank

- [x] 2.1 Add hotspot policy crossing a file's maximum unit rating with its windowed touch count, with a configurable minimum touch count defaulting to 5 and integer operands only.
- [x] 2.2 Insert the hot rank key after total triggered signals and the role class key after it, keeping every following key in its current order.
- [x] 2.3 Prove the rank remains a total, data-stable order and that primary debt precedes non-primary debt at equal rating while non-primary debt stays visible.
- [x] 2.4 Add pure tests at the minimum-touch boundary, for an unrated hot file, and for a rated cold file.

## 3. Architecture signals

- [x] 3.1 Add stable-dependency policy using integer cross-multiplication of degree operands with a minimum of 2 references, emitting a Watch architecture finding that retains both packages' exact operands.
- [x] 3.2 Add orphan file facts for supported primary files with zero verdict-graph incoming dependencies, exempting entry files by name and by manifest-declared entry.
- [x] 3.3 Add pure tests for the exact-equality non-violation case, the one-reference case, an entry file, and a file with one incoming dependency.

## 4. Knowledge concentration

- [x] 4.1 Emit a Watch evolutionary finding at 10 or more windowed commits with a top-contributor share of at least 90%, carrying package, contributor count, numerator, and denominator only.
- [x] 4.2 Add a kind discriminator to evolutionary findings so unexplained coupling and knowledge concentration stay distinguishable.
- [x] 4.3 Prove no contributor name, address, raw author field, or internal identifier reaches the report, terminal, or JSON.
- [x] 4.4 Add pure boundary tests at 9 and 10 commits and at 89% and 90% share using integer comparison.

## 5. Measurements

- [x] 5.1 Expose maximum nesting depth per rated unit from the existing nesting events, counted from zero at the unit body.
- [x] 5.2 Add parameter count per rated unit through the shared language contract with a default for languages without parameters.
- [x] 5.3 Keep both measurements collected and unrated in this change and record the promotion as owned by `adopt-report-schema-v4`.
- [x] 5.4 Add exact fixtures for both measurements in all 11 supported grammars, including Vue script, script-setup, and template regions.
- [x] 5.5 Add file size rating against 400 Watch and 800 High and container size rating against 300 Watch and 600 High on exclusive statement totals, both configurable.
- [x] 5.6 Add pure threshold-boundary tests for both size signals.

## 6. Flow, evidence, and gates

- [x] 6.1 Derive every new table inside existing passes with no new file read, traversal, or Git process.
- [x] 6.2 Order every new table by data-stable keys and re-prove serial and parallel byte equality.
- [x] 6.3 Prove work counts, allocation behavior, and performance gates stay inside their reviewed budgets.
- [x] 6.4 Add generated fixture evidence for hotspots, stable-dependency findings, knowledge concentration, size findings, orphan files, and window coverage fields.
- [x] 6.5 Prove JSON version 3 shape and terminal sections, labels, and vocabulary are unchanged and that every differing terminal byte is explained by the new rank keys.
- [x] 6.6 Update the documented rank sequence to include the hot and role class keys.
- [x] 6.7 Re-run the release binary on smackdebt, swiftide, and fluyt and record that production debt now outranks test debt and that hot files are identifiable.
- [x] 6.8 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [ ] 6.9 Archive this change before implementing `add-verdict-policy`.
