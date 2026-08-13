## Context

Discovery already assigns each selected source file to one package. Language
analysis already visits syntax that contains imports, uses, requires, includes,
and module declarations. Project orchestration already owns source reads and
composition. These seams can produce an internal dependency graph without
allowing parser code to access the filesystem or moving graph policy into an
adapter.

Static references are not equally resolvable across languages. Relative module
paths can usually be resolved from repository facts, while runtime imports,
macros, generated source, aliases, classpaths, and build-system behavior may be
unclear. Architecture findings must fail safely when identity is uncertain.

## Goals / Non-Goals

**Goals:**

- Build one explainable internal file graph and derived package graph.
- Keep language syntax, filesystem facts, resolution, graph policy, and output
  in separate owners.
- Detect dependency cycles and expose fan-in, fan-out, and instability.
- Compare architecture before and after a worktree change.
- Integrate code and architecture facts without one blended score.
- Preserve complete local processing and deterministic serial/parallel output.

**Non-Goals:**

- A cross-language call graph or compiler-grade type resolution.
- Runtime tracing, build execution, or user-provided resolver code.
- Treating external packages as internal graph nodes.
- Assigning arbitrary health limits to ordinary degree values.
- Inferring an internal edge when several targets remain plausible.
- A separate architecture command.

## Decisions

### Extract syntax locally and resolve centrally

Each concrete language implementation emits `DependencySyntax` values with the
reference kind, source span, raw target text, relative or absolute intent, and
language-owned resolution candidates. The language implementation owns module
naming, supported extensions, index-file conventions, import forms, and syntax
exceptions. It performs no path lookup.

Project orchestration supplies a read-only repository index built from discovery
paths, package identities, and inspected project configuration. Resolution
checks candidates against that index. Supported alias or project-root rules are
read from known configuration formats as data; configuration never executes
commands or loads code.

A reference resolves only when exactly one internal candidate matches. Zero
matches become external or unresolved according to language meaning. Several
matches become ambiguous. Unresolved and ambiguous references carry their source
location and reason into coverage facts.

### Store file edges once and derive package edges

Analysis owns flat `DependencyEdge` values linking source and target file
indexes. Duplicate references between the same files retain one edge with a
reference count and representative source locations. Self-file references do
not create graph edges.

Package edges are derived from unique cross-package file edges. An edge stores
the number of contributing file pairs and references. Dependencies within a
package remain available for file-level cycles and detail but do not create a
package self-edge.

External references are retained as descriptive dependency coverage with their
package names where safe. They do not participate in internal cycles, fan-in,
fan-out, or instability.

### Keep graph algorithms independent

Graph policy lives in `smackdebt-analysis` with one focused module per
algorithm:

- `strongly_connected_components.rs` finds components in linear graph time;
- `cycle_witness.rs` produces one stable shortest available witness per
  component for explanation;
- `dependency_degree.rs` calculates unique incoming and outgoing neighbors;
- `instability.rs` calculates outgoing divided by incoming plus outgoing;
- `architecture_comparison.rs` compares edges, cycles, and affected packages.

Algorithms consume indexed flat graphs and do not inspect parsers, paths, Git,
or output policy. Stable path order breaks graph traversal ties so serial and
parallel construction produce identical results.

### Rate cycles and describe other graph facts

A package component containing more than one package, or a package self-cycle
through a file cycle, is a High architecture finding. The finding owns one
stable witness and links all involved packages and file edges. File-only cycles
inside a package are Watch findings because they increase local change risk but
do not cross package responsibility.

Fan-in, fan-out, instability, dependency counts, external counts, unresolved
counts, and ambiguous counts remain exact descriptive facts. They order and
explain architecture detail but do not receive fixed health ratings in this
change.

Code findings and architecture findings have separate counts and sections. A
package can therefore have healthy functions and a High architecture cycle, or
complex functions with no static architecture finding, without either result
hiding the other.

### Compare complete before and after graphs

Diff analysis extracts dependency facts for every changed source side and reuses
unchanged inventory edges needed to build a complete affected graph. It does not
infer cycle behavior from changed files alone.

An introduced package cycle is Worse; a removed package cycle is Better. Added
or removed ordinary edges, degree changes, and file-only cycle changes are
Changed unless they create or remove a rated finding. Rename identity is applied
before comparison. Unclear resolution produces a diagnostic rather than a
guessed architecture direction.

### Integrate architecture into current reports

The default terminal flow keeps its code quality summary and adds an
`ARCHITECTURE` section with dependency coverage, cycle counts, leading affected
areas, and concise witnesses. The diff flow adds `ARCHITECTURE CHANGE` with
Worse, Better, and Changed architecture outcomes. Code and architecture each
have their own progressive drill target.

Path selection shows architecture facts involving the selected file, directory,
or package. Incoming edges from outside the selection remain visible because
they explain the selected area's pressure; unrelated graph regions are omitted
from terminal presentation. The retained root report and JSON remain complete.

JSON version 2 adds file edges, package edges, external dependency summaries,
resolution diagnostics, package graph measurements, architecture findings, and
architecture comparisons as flat indexed tables.

## Risks / Trade-offs

- **Resolution claims exceed available facts** → Require one matching target and
  retain unresolved or ambiguous reasons.
- **Diff graph misses an unchanged return path** → Build the affected complete
  graph from unchanged inventory facts plus before and after changed edges.
- **Cycle output is unstable** → Sort nodes and neighbors by stable path identity
  and define witness tie-breaking.
- **High-degree packages dominate output without clear harm** → Keep degree and
  instability descriptive until measured evidence supports policy.
- **Graph storage grows with imports** → Deduplicate file pairs, reserve edge
  tables, and measure generated dense and sparse repositories.
- **Integrated reports become noisy** → Keep code and architecture sections
  separate with independent concise defaults and complete JSON.

## Migration Plan

1. Add flat dependency and graph values plus pure algorithm tests.
2. Add language dependency syntax fixtures without changing product output.
3. Add repository indexing and exact internal resolution for supported forms.
4. Build codebase file and package graphs and add cycle findings.
5. Add complete before/after graph comparison.
6. Integrate terminal and JSON version-2 output.
7. Add black-box mixed-language graph fixtures and performance workloads.
8. Update documentation and run the complete gate.

Rollback removes architecture tables and presentation while leaving source
analysis unchanged. There is no persisted graph or user data migration.

## Open Questions

None.
