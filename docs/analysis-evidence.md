# Analysis evidence map

Each accepted spec area has a named proof owner. Focused tests establish exact
policy and adapter behavior. CLI acceptance then proves the same facts compose
through the installed product seam.

| Accepted spec | Evidence owner |
| --- | --- |
| `replace-source-analysis-engine/specs/language-analysis/spec.md` | `language_fixtures` exact grammar cases and `unified_acceptance::every_supported_language_matches_the_public_fact_manifest` |
| `replace-source-analysis-engine/specs/metric-semantics/spec.md` | analysis metric module truth tests and `language_fixtures` |
| `replace-source-analysis-engine/specs/analysis-performance/spec.md` | parser reuse tests, worker byte parity, and generated source baselines |
| `replace-source-analysis-engine/specs/workspace-architecture/spec.md` | dependency, entry-module, and API snapshot checks |
| `replace-source-analysis-engine/specs/architecture-documentation/spec.md` | architecture documentation review and architecture check |
| `replace-source-analysis-engine/specs/product-documentation/spec.md` | README command and language fact tests |
| `replace-source-analysis-engine/specs/progressive-exploration/spec.md` | codebase path snapshots and JSON index audit |
| `replace-source-analysis-engine/specs/report-schema-v2/spec.md` | schema validation, semantic assertions, and exact JSON results |
| `add-static-architecture-analysis/specs/architecture-analysis/spec.md` | graph truth tests, resolver fixtures, and static CLI fact manifest |
| `add-static-architecture-analysis/specs/analysis-performance/spec.md` | sparse, dense, package-count, and dependency-diff baselines |
| `add-static-architecture-analysis/specs/workspace-architecture/spec.md` | dependency direction and API snapshot checks |
| `add-static-architecture-analysis/specs/architecture-documentation/spec.md` | architecture check and documented graph limits |
| `add-static-architecture-analysis/specs/product-documentation/spec.md` | README architecture sections and executable examples |
| `add-static-architecture-analysis/specs/progressive-exploration/spec.md` | package and file drill snapshots, including incoming edges |
| `add-static-architecture-analysis/specs/report-schema-v2/spec.md` | schema validation, static semantic facts, and nested index rejection |
| `add-evolutionary-architecture-analysis/specs/evolutionary-analysis/spec.md` | history parser and aggregation truth tests plus evolution fact manifest |
| `add-evolutionary-architecture-analysis/specs/analysis-performance/spec.md` | evolution workload record, process counts, and worker parity |
| `add-evolutionary-architecture-analysis/specs/architecture-documentation/spec.md` | documented Git ownership, privacy, and history limits |
| `add-evolutionary-architecture-analysis/specs/product-documentation/spec.md` | README evolution and privacy sections |
| `add-evolutionary-architecture-analysis/specs/progressive-exploration/spec.md` | evolution package and diff drill snapshots |
| `add-evolutionary-architecture-analysis/specs/report-schema-v2/spec.md` | evolution JSON result, identity-key rejection, and index audit |
| `prove-unified-analysis/specs/end-to-end-evidence/spec.md` | `unified_acceptance` real-process matrix and committed results |
| `prove-unified-analysis/specs/analysis-performance/spec.md` | correctness-first complete-flow baselines and public worker parity |
| `prove-unified-analysis/specs/architecture-documentation/spec.md` | test-layer and acceptance-seam sections in `ARCHITECTURE.md` |
| `prove-unified-analysis/specs/product-documentation/spec.md` | executable README example test |
| `prove-unified-analysis/specs/report-schema-v2/spec.md` | codebase and diff schema checks, semantic assertions, exact bytes, and work counters |
| `prove-unified-analysis/specs/release-readiness/spec.md` | `release-evidence`, installed-command smoke, licenses, and complete gates |

The architecture check verifies that every spec file above remains present in
this map. Adding or renaming an accepted spec therefore requires an explicit
evidence decision.
