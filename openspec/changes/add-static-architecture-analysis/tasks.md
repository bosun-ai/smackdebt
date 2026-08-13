## 1. Graph domain and algorithms

- [ ] 1.1 Add typed file-edge, package-edge, resolution, graph-measurement, architecture-finding, and architecture-comparison values in analysis
- [ ] 1.2 Implement and test strongly connected components over flat indexed graphs with stable traversal order
- [ ] 1.3 Implement and test stable cycle witnesses, including package and file-only cycles
- [ ] 1.4 Implement and test unique fan-in, fan-out, instability, and package edge aggregation
- [ ] 1.5 Implement and test architecture comparison directions for introduced and removed cycles and ordinary edge changes

## 2. Language dependency syntax

- [ ] 2.1 Extend the private language contract with dependency syntax and resolution-candidate values without adding I/O
- [ ] 2.2 Add exact dependency fixtures for Rust, Ruby, JavaScript, JSX, TypeScript, TSX, Vue, Python, Java, C, and C++
- [ ] 2.3 Cover relative, root, package, external, dynamic, malformed, and unsupported reference forms with original source spans
- [ ] 2.4 Prove dependency extraction shares the existing source parse and node traversal

## 3. Resolution and graph construction

- [ ] 3.1 Build one project-owned repository index from discovered paths, package identities, and supported project configuration
- [ ] 3.2 Resolve exactly one internal candidate, retain external references, and report unresolved or ambiguous references without guessing
- [ ] 3.3 Deduplicate file pairs and derive package edges with exact contributing file-pair and reference counts
- [ ] 3.4 Build complete codebase graphs while preserving one inventory walk and one current-source read
- [ ] 3.5 Build complete affected before and after diff graphs with rename handling and fixed Git process counts

## 4. Reports and output

- [ ] 4.1 Add separate code and architecture summaries, counts, findings, links, and comparison facts to the completed report
- [ ] 4.2 Add concise default and complete `--all` architecture sections to codebase and diff terminal output
- [ ] 4.3 Add progressive path selection that retains explanatory incoming and outgoing architecture edges
- [ ] 4.4 Extend JSON version 2 and its checked schema with complete static architecture tables
- [ ] 4.5 Preserve direct streamed output and byte-identical serial and parallel reports

## 5. Evidence and documentation

- [ ] 5.1 Add a deterministic mixed-language repository with internal, external, unresolved, ambiguous, cyclic, and acyclic dependencies
- [ ] 5.2 Add black-box codebase and diff snapshots for introduced and removed package cycles, file cycles, degree facts, and path drill-down
- [ ] 5.3 Add sparse, dense, many-package, and large-diff graph workloads with correctness checks and measured memory and wall-time limits
- [ ] 5.4 Update README and architecture documentation with graph meaning, resolution limits, integrated output, and JSON version-2 fields
- [ ] 5.5 Run focused graph and language tests, complete black-box fixtures, formatting, linting, workspace tests, architecture checks, API snapshots, licenses, performance checks, strict OpenSpec validation, and diff checks
