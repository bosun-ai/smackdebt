## Context

Package identity in Smackdebt is rooted in directories: a package is the
directory that owns a recognized manifest. Dependency resolution matches
candidate paths against discovered files. Neither step ever reads the name a
package declares for itself, so a cross-package import written against the
declared name (`smackdebt_analysis`, `@acme/ui`, `swiftide-core`) has no
candidate that can match and falls through to `external`.

Every downstream architecture signal is derived from the package graph, so the
missing name mapping silently disables package cycles, degree, instability, and
the `no code dependency` claim on coupling findings. The field runs on
smackdebt, swiftide, and fluyt show all four symptoms at once.

This is the first change in the v-next set because the three later analysis and
presentation changes all describe signals computed from this graph.

## Goals / Non-Goals

**Goals:**

- Resolve cross-package references by declared manifest name for the manifest
  kinds Smackdebt already recognizes.
- Keep a genuinely external dependency external when its name is ambiguous.
- Make the coupling `no code dependency` claim true.
- Emit one coupling row per package pair and no meaningless ancestor-descendant
  pairs.
- Stop emitting non-dependency Rust tokens as dependency targets.

**Non-Goals:**

- Reading, executing, or emulating build configuration, lockfiles, resolver
  algorithms, workspace inheritance, path or version constraints, aliases, or
  `tsconfig` path mappings.
- Serializing the manifest name in JSON; that field arrives with schema
  version 4 in `adopt-report-schema-v4`.
- Adding new findings, ratings, verdicts, or terminal sections.

## Decisions

### Discovery extracts the declared name during manifest recognition

Discovery already opens a manifest to recognize a package root. It reads the
declared name in the same read and stores it on the package record:

- `Cargo.toml`: `[package] name`, overridden by `[lib] name` when present.
- `package.json`: `name`, including a scoped `@scope/name` value stored whole.
- `pyproject.toml`: `[project] name`.
- Gemspec: the gemspec's declared name.

A missing, empty, or unreadable name leaves the package without a manifest name
and is not an error. No second walk, no extra read, and no new process.

### Resolution consults a manifest-name index only after path candidates fail

Path-candidate resolution is unchanged and still wins. Only a reference that
would otherwise be classified external is normalized and looked up:

- The first path segment of the reference is taken (`smackdebt_analysis` from
  `smackdebt_analysis::report::ReportBuilder`, `@acme/ui` from
  `@acme/ui/button`).
- Rust normalizes hyphens and underscores to one form on both sides. Other
  languages compare the declared name exactly.
- The lookup key maps to internal packages. The reference resolves internally
  only when exactly one internal package matches.

The result is a package-level `uses` edge. When the matched package has a
resolvable entry file, the edge also carries that file-level target so file and
package graphs agree; otherwise the edge is package-scoped and the file target
stays absent.

### Shadowing keeps the diagnostic instead of guessing

If a declared manifest name maps to more than one internal package, the
reference is ambiguous: no internal edge is created and the ambiguity
diagnostic is retained with its source location. A repository that depends on a
published package whose name equals an internal package name therefore resolves
internally only when that name is unique inside the repository, which is the
same exactly-one-match rule the path candidates already use. The diagnostic
trail stays in the report so a false internal match is reviewable.

### One coupling row per package pair

Coupling currently keys rows by package pair plus role and trust, and the human
report cannot tell the variants apart, so one pair prints three times with
different numbers. The retained coupling row becomes one row per unordered
package pair. The eligible parsed aggregate that already feeds findings stays
the finding operand; role and trust variants are aggregated into that single row
rather than emitted as sibling rows, so a rendered pair has exactly one set of
operands. Descriptive role and trust evidence remains available per package in
package history rows, which is where per-role evidence is meaningful.

### Ancestor-descendant scope pairs are not coupling pairs

A pair whose one endpoint is an ancestor scope of the other (`repository root ↔
crates/project`) cannot be evidence of hidden coupling: the ancestor contains
the descendant, so shared commits are structural. Such pairs are excluded before
similarity is computed, in both descriptive rows and findings.

### Rust extraction emits dependency targets only

Rust dependency syntax translation treats visibility tokens (`pub`, `pub(crate)`
and its variants) as modifiers of the item, never as the first path segment of a
dependency target. A `pub use a::b;` reference emits the target `a::b`.

## Risks / Trade-offs

- **False internal matches.** A repository can depend on a published package
  that shares an internal package's declared name. The unique-match rule keeps
  the match only when the name is unambiguous inside the repository, and every
  unmatched or ambiguous reference keeps its diagnostic.
- **Large value churn in evidence.** Externals fall, internal edges rise, cycles
  and instability appear, and coupling findings can disappear because a real
  dependency now explains them. Every regenerated snapshot is reviewed against
  the field runs.
- **Coupling aggregation loses per-role rows.** JSON and `--all` keep exact
  operands for the single retained row; role-specific history evidence remains
  in package history rows.
- **Manifest parsing surface.** Only declared-name fields are read; no build
  configuration is executed and no resolver semantics are emulated.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. Add discovery fixtures for each manifest kind, including a scoped
   `package.json` name and a `[lib] name` override.
3. Add pure resolution tests for unique match, hyphen and underscore
   normalization, ambiguous shadowing, and entry-file fallback.
4. Implement extraction, resolution, coupling de-duplication, the
   ancestor-descendant exclusion, and the Rust extraction fix.
5. Regenerate and review terminal and JSON evidence; JSON version 3 structure
   must be unchanged.
6. Re-run the release binary on smackdebt, swiftide, and fluyt: the
   `crates/analysis ↔ crates/project · no code dependency` claim must be gone
   and unmatched imports on smackdebt must fall from 29 to near zero.
7. Archive this change before implementing `deepen-debt-signals`.

Rollback restores the previous resolution behavior and evidence bytes; no data
migration is required because the JSON structure does not change.
