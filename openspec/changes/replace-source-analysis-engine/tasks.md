## 1. Proof baseline and contracts

- [ ] 1.1 Replace timestamp-derived temporary test paths with collision-safe fixtures and pass the full parallel workspace suite repeatedly
- [ ] 1.2 Add exact semantic event fixtures and pure tests for cognitive complexity, cyclomatic complexity, and logical lines
- [ ] 1.3 Add black-box terminal and JSON version-2 fixture infrastructure with explicit reviewed snapshot updates
- [ ] 1.4 Strictly validate this complete OpenSpec change before source implementation

## 2. Generic engine

- [ ] 2.1 Add private semantic value objects and the private generic `Language` trait without exposing parser types across the crate seam
- [ ] 2.2 Add compiled generic dispatch, one iterative unit walker, and independent algorithm state
- [ ] 2.3 Reuse parser, query cursor, traversal, and result capacity per project worker
- [ ] 2.4 Construct analysis-owned file and unit facts directly and remove the duplicate raw analysis model
- [ ] 2.5 Prove nested units, parse recovery, original spans, one parse per region, and serial/parallel identity at the language crate seam

## 3. Language migration

- [ ] 3.1 Implement and enable Rust through exact unit, metric, recovery, and span fixtures
- [ ] 3.2 Implement and enable Ruby methods, singleton methods, closures, containers, modifiers, and recovery through the same shared algorithms
- [ ] 3.3 Implement and enable JavaScript, JSX, TypeScript, and TSX, including methods, closures, class fields, and embedded JSX control flow
- [ ] 3.4 Implement and enable Vue document regions, script injection, template control flow, style coverage, and original positions
- [ ] 3.5 Implement and enable Python and Java through exact nested-control-flow and logical-statement fixtures
- [ ] 3.6 Implement and enable C and C++ through exact functions, methods, namespaces, lambdas, preprocessing recovery, and logical-statement fixtures
- [ ] 3.7 Keep Kotlin visible as unsupported and add a proof that unsupported source never contributes healthy units

## 4. Schema and removal

- [ ] 4.1 Implement the flat JSON schema version-2 source report and checked JSON Schema
- [ ] 4.2 Replace version-1 terminal and JSON acceptance expectations with reviewed version-2 expectations
- [ ] 4.3 Remove `rust-code-analysis`, obsolete parser dependencies, text scanners, compatibility branches, and intermediate measurement mappings
- [ ] 4.4 Refresh compiler-visible API snapshots and dependency-direction checks

## 5. Documentation and complete evidence

- [ ] 5.1 Update the README with exact metric rules, corrected examples, supported-language evidence, and JSON version 2
- [ ] 5.2 Update the architecture guide with the private trait, language ownership, generic traversal, algorithm modules, parser lifetimes, and migration rule
- [ ] 5.3 Run focused language tests, black-box source fixtures, serial/parallel byte comparisons, allocations, and generated mixed-language performance workloads
- [ ] 5.4 Run formatting, warning-free linting, all workspace tests, architecture checks, API snapshots, licenses, performance checks, strict OpenSpec validation, and diff checks
