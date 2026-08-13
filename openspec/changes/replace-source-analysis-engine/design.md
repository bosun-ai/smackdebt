## Context

The language crate currently dispatches supported files to an upstream metric
engine, an owned Ruby line scanner, or an owned Vue line scanner. The upstream
engine builds a tree-sitter tree and calculates a broad metric suite, then
Smackdebt retains only cognitive complexity, cyclomatic complexity, and logical
lines. Ruby and Vue derive the same reported fields from text patterns with
different meanings.

Tree-sitter supplies grammar-specific concrete syntax trees. It does not supply
one shared semantic model. The design therefore needs language-local syntax
classification and shared algorithms without building a second owned syntax
tree or allowing algorithm code to know concrete grammars.

## Goals / Non-Goals

**Goals:**

- Give each supported language one private implementation that owns all syntax
  knowledge.
- Give each retained measurement one language-independent implementation.
- Define exact metric behavior through readable fixtures.
- Parse each language region once and reuse worker-local state.
- Preserve flat Smackdebt facts, original source spans, deterministic output,
  and visible coverage failures.
- Remove the upstream engine and duplicate intermediate analysis values.

**Non-Goals:**

- Runtime plugins, dynamic dispatch, or a public analyzer API.
- One normalized owned AST shared by every language.
- A call graph, recursion scoring, type checking, or full symbol resolution.
- Supporting Kotlin before its own exact fixtures pass.
- Preserving current metric values when defined semantics show they are wrong.
- Supporting JSON schema versions 1 and 2 at the same time.

## Decisions

### Use a private generic language trait

`smackdebt-languages` defines a private `Language` trait implemented by concrete
language marker types. Compiled dispatch selects a concrete call such as
`analyze::<Rust>` or `analyze::<Ruby>`; no trait object or callback registry is
stored.

The trait supplies meaningful syntax values for:

- unit boundaries, identities, containers, kinds, bodies, and original spans;
- control-flow events, alternatives, boolean operators, and nesting effects;
- logical statements and exclusions;
- dependency syntax and document injection regions needed by later changes;
- parse recovery and language-specific exceptions.

Concrete implementations may inspect their tree-sitter node kinds, fields,
queries, and source text. Shared algorithm modules may use only trait-provided
values. They may not match a concrete language identifier, grammar node name,
or language module.

The trait and tree-sitter types remain private to the language crate. The
existing public seam continues to return analysis-owned `FileAnalysis` values.

### Traverse once and feed independent algorithms

One iterative depth-first traversal visits each node in a rated unit. The
language implementation classifies that node once into meaningful syntax. The
walker then feeds borrowed events to independent state owned by:

- `cognitive_complexity.rs`;
- `cyclomatic_complexity.rs`;
- `logical_lines.rs`.

The algorithms do not call one another and do not share a generic child-total
rule. A nested rated unit is analyzed separately and its subtree is skipped
while measuring its parent. This produces direct complexity and exclusive
logical lines without subtracting a completed child result later.

Worker-local analyzer state retains one tree-sitter parser per grammar used by
that worker, reusable query cursors, traversal stacks, and result capacity.
Source and syntax trees are released after the file result is reduced to
Smackdebt facts.

### Define metric meaning before compatibility

Cognitive complexity starts at zero and adds cost for structural control-flow
breaks, nesting, alternatives, labeled jumps where the language supports them,
and changes between boolean-operator runs. Else-if behavior, match or switch
arms, exception handlers, closures, and nested functions are described by each
language through the shared event vocabulary. Recursion is not counted because
the source tree alone cannot establish a reliable cross-language call graph.

Cyclomatic complexity starts at one per rated unit and adds one for each
language-defined independent decision event. Tests distinguish structural
branches, boolean decisions, handlers, comprehensions, and language constructs
that look similar but do not create another path.

Logical lines count language-defined executable or declarative statements in a
rated unit. Blank lines, comments, wrapper declarations, markup, and nested
rated units do not count. Several statements on one physical line count
separately, while one statement split across physical lines counts once.

Exact truth tables and fixture snapshots define these rules. Migration tests
record intentional differences from the current engine; current values are not
used as expected results merely because they already exist.

### Treat Vue as a document with injected languages

The Vue implementation parses the document grammar, identifies script,
template, and style regions, and uses included source ranges for JavaScript or
TypeScript script analysis. Expression-valued template directives use the
matching script-language expression parser where needed. Every result retains
its position in the original Vue file.

Template conditions, loops, conditional expressions, and boolean decisions
produce shared control-flow events. A plain event-handler reference does not
create a branch. Style regions count toward covered source but create no rated
unit.

### Move directly to JSON schema version 2

Schema version 2 retains flat tables and typed indexes for paths, scopes, files,
code findings, comparisons, health, activity, and diagnostics. Names and source
locations remain human-readable while relationships use indexes. Healthy units
remain aggregate counts rather than retained unit records.

The version field is `2`; `--json` has no version selector. Version 1 fixtures
are replaced rather than maintained. Later architecture changes extend the
version-2 model with new tables without changing the meaning of source facts.

### Replace languages in dependency order

The engine is proven with Rust and Ruby first because their syntax and unit
shapes differ. JavaScript, JSX, TypeScript, and TSX follow so Vue can depend on
their owned implementations. Python and Java follow, then C and C++. One
language is enabled only after exact syntax, metric, recovery, span, nested-unit,
serial/parallel, and performance fixtures pass.

The upstream dependency remains available only for languages not yet switched
during the change. It is removed after the final registry entry moves. Product
documentation lists a language as supported only while its active engine passes
the same proof requirements.

## Risks / Trade-offs

- **The trait becomes a generic parser wrapper** → Keep it private and expose
  meanings rather than raw nodes or parser operations.
- **Queries and grammar versions drift** → Pin grammar versions and fail exact
  language fixtures when node shapes change.
- **One traversal couples algorithms** → Keep algorithm state independent and
  test each algorithm directly from semantic event fixtures.
- **Corrected metrics surprise current users** → Publish rule examples and
  before/after fixture notes before the first release.
- **Vue injection loses source identity** → Use original byte and point ranges
  and verify every emitted span through complete document fixtures.
- **The engine regresses performance** → Measure complete CLI flows, parser
  reuse, allocation counts, wall time, and peak memory before replacing the
  existing baselines.

## Migration Plan

1. Restore a reliable full-workspace test baseline and add the new black-box
   fixture harness without changing product output.
2. Add semantic value objects, generic algorithm tests, and the private trait.
3. Move Rust and Ruby and compare their exact truth fixtures.
4. Move the JavaScript family, then Vue.
5. Move Python and Java, then C and C++.
6. Remove the upstream engine and duplicate intermediate analysis values.
7. Switch JSON output and documentation to version 2.
8. Refresh API snapshots and measured performance baselines, then run the full
   gate.

Rollback switches incomplete language entries back to the temporary upstream
adapter before it is removed. After removal, rollback is a source revert; no
user data is migrated.

## Open Questions

None.
