## 1. Requirement text

- [ ] 1.1 Rewrite `Codebase child order is severity-led` so it states the child area row keys (High descending, Watch descending, name ascending) that `codebase_child_order` implements.
- [ ] 1.2 Defer finding order to the finding rank owned by `hotspot-analysis` by reference, with no key enumeration to drift.
- [ ] 1.3 Replace the stale activity scenario with hot-aware and role-aware scenarios that agree with the accepted rank.

## 2. Close

- [ ] 2.1 Pass `openspec validate --all --strict` with this change active, then archive it.
- [ ] 2.2 Pass `just check`.
