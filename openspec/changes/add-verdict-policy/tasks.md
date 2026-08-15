## 1. Codebase verdict

- [ ] 1.1 Add a pure verdict module in analysis owning the frozen tier ids `empty`, `clean`, `solid`, `worn`, `fights_back`, and `lost` and their exact sentences.
- [ ] 1.2 Implement tier mapping by integer permille of High count over checked units with boundary values belonging to the lower tier and no floating-point value anywhere in the path.
- [ ] 1.3 Implement the architecture escalation floors at one and three High architecture findings so a floor never lowers a tier.
- [ ] 1.4 Add pure tests for zero checked units, no debt, watch-only, exactly 1%, just above 1%, exactly 5%, just above 5%, both floors, and a floor applied to an already higher tier.

## 2. Diff verdict

- [ ] 2.1 Add the frozen diff tier ids `no_debt_change`, `better`, `worse`, and `mixed` with their exact sentences.
- [ ] 2.2 Reconcile the tier across source, architecture, and evolutionary comparisons in one decision.
- [ ] 2.3 Retain per-family counts so the facts can name the family that moved and label every count with its word, including zero counts.
- [ ] 2.4 Add the full diff truth table as pure tests, including the contradiction case where no source comparison moved and a package cycle was introduced.

## 3. DebtDiffSelection

- [ ] 3.1 Own per-scope typed-ID lists for Regressed, Improved, Added at Watch or High, Removed at Watch or High, and MetricChanged-while-rated source comparisons.
- [ ] 3.2 Include architecture and evolutionary findings introduced or removed in the same per-scope selection.
- [ ] 3.3 Exclude healthy added or removed units, unchanged comparisons, ambiguous comparisons, and fixture or generated source from the selection while keeping them in JSON.
- [ ] 3.4 Feed the selection to the verdict, to presentation, and to index-integrity audits that reject duplicate IDs within one scope.
- [ ] 3.5 Add pure tests for each membership rule and for a scope whose only changes are healthy additions.

## 4. Worst offender and scope verdicts

- [ ] 4.1 Select the first ranked finding for the scope with its resolved path and reason `hot AND complex` or `most complex`.
- [ ] 4.2 Fall back to the first package-cycle witness with reason `package dependency cycle` and produce no worst offender when neither exists.
- [ ] 4.3 Complete the root verdict during report construction and expose a pure verdict function for any selected scope.
- [ ] 4.4 Add pure tests for both reasons, the fallback, the absent case, and a path scope whose verdict differs from the root verdict.

## 5. Evidence and gates

- [ ] 5.1 Add generated fixture evidence for every codebase tier, both escalation floors, all four diff tiers, the contradiction case, and worst-offender fallback.
- [ ] 5.2 Prove serial and parallel runs produce identical verdicts, counts, selections, and worst offenders.
- [ ] 5.3 Prove no renderer computes a verdict and no configuration surface is introduced.
- [ ] 5.4 Prove terminal bytes and JSON version 3 bytes are unchanged by this change, since rendering and serialization land in the two following changes.
- [ ] 5.5 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [ ] 5.6 Archive this change before implementing `redesign-terminal-report`.
