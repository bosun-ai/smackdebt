## Context

Version 3 is a normalized fact dump: flat indexed tables joined by index. That
shape is right for detail and wrong for the first question. A consumer that only
wants "is this code shit?" has to parse megabytes and join tables to find out,
and an LLM reading the JSON pays for all of it.

The four earlier v-next changes added a completed verdict, hotspots, size
findings, orphan files, stable-dependency findings, knowledge concentration,
window coverage, and manifest names. Serializing them is a breaking change, so
it happens once, in one version bump, together with the two deferred rating
promotions and the float cleanup.

## Goals / Non-Goals

**Goals:**

- Answer the common question at the top of the object with zero joins.
- Serialize every fact the report now owns.
- Serialize integers and strings only.
- Promote nesting and parameter count to rated signals where their thresholds
  and their serialization land together.
- Leave exactly one machine contract in accepted specs.

**Non-Goals:**

- Keeping a version-3 compatibility object or a schema selector. The workspace
  is unpublished; there is no consumer to migrate.
- Changing terminal sections or vocabulary.
- Changing analysis policy beyond the two promoted signals.

## Decisions

### The head is denormalized on purpose

The object opens with:

- `verdict`: `tier` (the frozen id), `sentence` (the analysis-owned bytes), and
  `mode` (codebase or diff).
- `summary`: checked, high, and watch counts; the debt-diff counts labeled by
  their words; and `worst`, up to three fully resolved entries carrying path
  strings, identity, and reason.

The head duplicates facts that also exist in the tables. That redundancy is the
feature: it is bounded, it is derived from the same completed report, and it is
what makes `smackdebt --json | head` useful. Duplication is stated in the schema
so a consumer knows it can trust either source.

### New tables and discriminators

- `hotspots`: file index, maximum unit rating, touch count.
- `size_findings`: file index, subject (file or container), container name for a
  container finding, measured value, and the rating that triggered.
- `orphan_files`: file index only, as a descriptive table.
- `stable_dependency_findings`: its own table, because Tasks 2-4 gave the family
  its own identity type. It carries kind `stable_dependency_violation`, both
  packages' integer degree operands, the reference count, and witness edges.
- `knowledge_concentration_findings`: its own table for the same reason,
  carrying kind `knowledge_concentration` and counts without identity.
- `evolutionary_findings[].kind` and `architecture_findings[].kind`: each names
  its own family, so a consumer merging the four finding tables keeps them
  apart.
- `comparisons[].start_line` and `[].end_line`: the located side of a
  comparison, nullable, so JSON states the `path:line` the terminal prints.
- `history_coverage`: gains the window length in days and the count of streamed
  commits excluded by the window.
- Package records: gain `manifest_name`, absent when a manifest declares none.

### Floats leave the contract

`similarity` and `ratio` are removed from serialized output. Both were derived
from integer operands that remain present — shared and union commits for
similarity, numerator and denominator for concentration — so no information is
lost and every serialized number is exact. This also removes the last place
where the integer-only arithmetic rule was contradicted by the output.

### Rating promotion lands here

Maximum nesting depth becomes a rated signal at Watch 4 and High 7. Parameter
count becomes a rated signal at Watch 6 and High 9. Both thresholds are
configurable like the existing signals.

The promotion belongs in this change and not in `deepen-debt-signals` because a
rating must be explainable from the serialized measurements of the same report.
Rating them under version 3 would produce findings whose cause was not in the
machine output. The accepted rule that a rating is explainable from three
measurements is amended to five.

Every comparison re-derives with the new signals, and every rating that flips is
reviewed.

### Version 3 and version 2 are removed

The CLI never presented version 2, and version 3 is replaced outright. Both
capabilities are removed from accepted specs so exactly one machine contract
exists. Archived changes keep their history.

## Risks / Trade-offs

- **A breaking schema change.** Nothing is published, so the cost is snapshots
  and documentation, not consumer migration.
- **Head redundancy can drift from tables.** Both are produced from the same
  completed report in one pass and acceptance asserts they agree.
- **Rating promotion moves findings.** Nesting and parameter thresholds create
  new Watch and High findings; every flipped rating is reviewed and the
  boundaries have exact pure tests.
- **Removing floats changes what consumers read.** The operands remain, so a
  consumer computes the ratio if it wants one, at whatever precision it chooses.
- **Measurement arity ripple.** Rating two more measurements touches health
  policy, configuration, comparisons, and fixtures; it is confined to this
  change.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. Write the version-4 schema and the denormalized head first, with acceptance
   asserting head and table agreement.
3. Add the new tables, discriminators, window fields, and `manifest_name`.
4. Remove serialized `similarity` and `ratio` and prove no floating-point value
   remains in output.
5. Promote nesting and parameter count with pure threshold tests at 3/4, 6/7,
   5/6, and 8/9, then review every flipped rating and comparison.
6. Update `schemas/README.md`, `ARCHITECTURE.md`, `AGENTS.md`, and the README.
7. Regenerate every JSON snapshot and review each file.
8. Re-run the release binary on smackdebt, swiftide, and fluyt and read the head
   of each JSON result as the product review.
9. Archive this change; `prepare-first-release` may then resume.

Rollback restores version 3 and its schema. No published consumer exists, so no
compatibility shim is required.
