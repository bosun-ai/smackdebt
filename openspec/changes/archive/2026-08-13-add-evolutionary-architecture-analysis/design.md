## Context

Source metrics and static dependencies describe present structure. Git history
adds a different kind of evidence: where work accumulates, which packages move
together, and whether knowledge is concentrated. The Git crate already owns
repository processes and history parsing, while analysis owns policy and the
project crate owns composition.

History is incomplete by nature. Shallow clones, missing objects, binary
changes, path filters, and repositories without commits must remain visible as
coverage facts instead of producing invented certainty.

## Goals / Non-Goals

**Goals:**

- Derive exact churn, touch, change-coupling, and contributor-concentration
  facts from one streamed history read.
- Follow rename history for files that still exist in the selected inventory.
- Find recurring package relationships that static dependencies do not explain.
- Protect contributor privacy in terminal and JSON output.
- Preserve deterministic output and keep history optional when Git evidence is
  unavailable.

**Non-Goals:**

- A combined debt score or a predicted defect score.
- Judging individual contributors or displaying their identities.
- Including deleted files as current report units.
- Reconstructing every historical manifest or package boundary.
- Runtime tracing, issue-tracker data, blame output, or remote repository data.
- Claiming that a worktree change altered historical measurements.

## Decisions

### Stream one structured history source

`smackdebt-git` starts one history process for a report. It streams non-merge
commits with commit identity, normalized author identity, timestamp, rename
records, and textual added/deleted line counts. Structured arguments and record
separators are used; no shell is involved.

The stream is parsed incrementally. Project orchestration maps records to the
selected current inventory and sends compact commit facts to analysis. Source
text and full commit messages are never retained. A binary or otherwise
uncounted file change contributes a touch but not invented line churn.

The selected history window is explicit report metadata. The initial policy
uses all locally available non-merge history reachable from the analyzed
revision. A shallow repository reports that limitation. A future configurable
window can replace this policy without changing the measurement types.

### Anchor history to current file identity

The history stream is read from newest to oldest. Rename records extend an
alias chain from the current path to its earlier path, so touches and churn
before a rename belong to the current file. Each historical change contributes
to at most one selected current file.

Files that do not lead to a selected current file are excluded from current
file and package measurements. Current discovery owns package assignment;
Smackdebt does not pretend to know historical package boundaries that no longer
exist. Coverage records count excluded history and broken rename chains.

### Keep four exact measurements

Analysis owns one focused module for each algorithm:

- `churn.rs` sums textual lines added and deleted and counts commits touching
  each current file and package;
- `change_coupling.rs` counts one shared change per unordered package pair per
  commit and calculates Jaccard similarity as shared commits divided by commits
  touching either package;
- `contributor_concentration.rs` counts each normalized contributor once per
  package per commit and calculates the largest contributor touch count divided
  by all contributor touch counts for that package;
- `evolutionary_comparison.rs` attaches current worktree outcomes to unchanged
  historical context without comparing history to itself.

Package churn sums its selected files. Package touches count a commit once even
when several files changed. Coupling retains pairs with at least two shared
commits. Contributor count and concentration are descriptive facts, not health
ratings.

All ratios store their numerator and denominator beside the derived value.
Stable package order breaks ties. Algorithms consume typed indexes and compact
commit facts; they do not inspect Git, paths, source text, or output policy.

### Use coupling as evidence, not proof

A retained cross-package coupling pair becomes a Watch architecture finding
when no static package dependency exists in either direction. Its explanation
includes both packages, shared-commit count, union-commit count, similarity,
and the absence of a static edge.

Coupling that matches a static edge remains descriptive. High churn, many
contributors, few contributors, and high concentration remain descriptive in
this change because repository size and working style affect their meaning.
No evolutionary fact silently raises a code-metric or cycle rating.

### Protect author identity at the report seam

The Git adapter normalizes authors through repository mailmap data when
available and assigns an internal identity used only during aggregation. Names,
email addresses, raw author fields, and internal identifiers do not cross into
the retained report.

Reports contain only contributor counts, the top-contributor numerator and
denominator, and the resulting concentration. Diagnostics never quote an
author field. Privacy tests search terminal and JSON bytes for fixture names and
addresses.

### Present history as its own report section

Codebase output adds `EVOLUTION` after static architecture. Its concise view
shows history coverage, leading churn areas, unexplained coupling findings, and
contributor concentration. Code, static architecture, and evolution retain
separate finding counts.

Diff output treats history as context for the changed files and packages. It
shows their existing churn, coupling, and concentration beside source and
static-graph changes, but it does not label those historical values Better or
Worse. An evolutionary Watch finding can be introduced or removed only when
the worktree changes the static edge that explains an existing coupling pair.

JSON version 2 adds flat history coverage, file history, package history,
coupling, concentration, and evolutionary finding tables. No author table or
author field exists.

### Fail safely when history is unavailable

A directory outside Git, an empty repository, an unreadable object, or an
interrupted history stream does not erase source and static architecture
results. The report carries a history availability state and diagnostic.
Partial history is marked incomplete and is not presented as complete evidence.

## Risks / Trade-offs

- **Rename chains are incomplete** -> Count the gap and stop assigning earlier
  records rather than guessing identity.
- **Large commits create many package pairs** -> Deduplicate packages per
  commit, count each unordered pair once, and measure dense generated histories.
- **Repository age dominates totals** -> Preserve the exact history window and
  expose both touches and line churn rather than hiding scale in one score.
- **Contributor data leaks identity** -> Convert to aggregate values before the
  report seam and assert fixture identities are absent from output bytes.
- **History failures hide useful current analysis** -> Return source and static
  graph results with an explicit history diagnostic.
- **Coupling is mistaken for causation** -> Rate only recurrent unexplained
  relationships as Watch and explain the evidence used.

## Migration Plan

1. Add compact Git history records and generated parser fixtures.
2. Add rename identity, churn, coupling, and concentration value objects.
3. Add pure algorithms with hand-calculated truth tables.
4. Compose current-inventory history without changing output.
5. Join coupling to static package edges and create Watch findings.
6. Add codebase, diff, and JSON version-2 presentation.
7. Add privacy, partial-history, determinism, and process-count acceptance tests.
8. Update product and architecture documentation and run the complete gate.

Rollback removes history collection and evolutionary report tables. Source and
static architecture analysis continue to work, and no stored data requires a
migration.

## Open Questions

None.

