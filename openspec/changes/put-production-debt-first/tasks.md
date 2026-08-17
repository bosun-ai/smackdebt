## 1. Rank order

- [x] 1.1 Rewrite `finding_rank_uses_every_accepted_key_in_order` for the sequence rating, role class, hot, signals at rating, triggered signals, cognitive complexity, cyclomatic complexity, logical lines, activity, path, span, commenting every assertion with the key that decides it.
- [x] 1.2 Rewrite the total, data-stable order test so its expected sequence reads `["hot primary", "primary", "hot test", "test"]`.
- [x] 1.3 Add a test proving a primary finding that is not hot ranks above a non-primary finding that is hot at equal rating.
- [x] 1.4 Add a test proving hot state decides before signals at rating and before total triggered signals among primary findings.
- [x] 1.5 Add a test proving a repository whose only rated findings are non-primary still names a worst offender, with no worst-offender filter introduced.
- [x] 1.6 Reorder the `FindingRank` field declarations and its constructor to match, keeping the type `Ord`-derived and the order total and data-stable.
- [x] 1.7 Prove serial and parallel runs still produce byte-identical terminal and JSON output.

## 2. Added and removed diff cards

- [ ] 2.1 Add acceptance expectations for an added card showing the after-side measurements and a removed card showing the before-side measurements, each with its direction word first.
- [ ] 2.2 Add an acceptance expectation that an added unit whose measurements are all zero renders the bare direction word.
- [ ] 2.3 Render the present side's nonzero absolute measurements on added and removed comparison cards without adding arithmetic to the renderer.
- [ ] 2.4 Keep the unsafe-to-match card as its direct explanatory sentence alone.

## 3. Documentation

- [x] 3.1 Update the README rank sentence to the new sequence and replace `statements` with logical lines.
- [x] 3.2 State in the README that primary source precedes non-primary source at equal rating, that hot decides next, and that non-primary debt stays visible below it.
- [x] 3.3 Update the `ARCHITECTURE.md` rank paragraphs so the documented order matches the implemented order.
- [x] 3.4 Grep the README and `ARCHITECTURE.md` for stale rank wording and for `statements` used as a measurement name.

## 4. Evidence

- [x] 4.1 Regenerate the affected unified and acceptance snapshots and review each file for ordering-only movement, with no row added, removed, or recounted.
- [ ] 4.2 Regenerate the diff terminal snapshots carrying added or removed cards and confirm changed cards stay byte-identical.
- [x] 4.3 Confirm ratings, signals, measurements, verdict tiers, counts, exit codes, work counts, and the JSON schema shape are unchanged.

## 5. Field verification

- [x] 5.1 Build the release binary and run it on tokio, scikit-learn, opencode, kwaak, and fluyt.
- [x] 5.2 Record that every `worst:` line names production code: fluyt names `bow/src/views/ConfigureTaskRun.vue` (hot production; the cold `npm/resolver.rs` predicted while drafting stays below it because hot outranks cognitive complexity in the accepted order, unchanged by this proposal), kwaak names `src/frontend/app.rs` rather than a benchmark, and tokio names `tokio/src/sync/notify.rs` `poll_notified`.
- [x] 5.3 Record that benchmark and test findings rank below production findings everywhere while remaining visible.
- [ ] 5.4 Record that fluyt's diff added cards show measurements and that its changed cards are byte-identical to the previous run.

## 6. Close

- [ ] 6.1 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [ ] 6.2 Archive this change before authoring `classify-production-architecture`.
