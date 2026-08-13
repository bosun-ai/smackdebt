## ADDED Requirements

### Requirement: Codebase hierarchy uses discovery package assignment
Project orchestration SHALL pass each selected file's discovery-owned package
identity into hierarchy construction. Hierarchy construction SHALL NOT infer a
second package assignment from path prefixes.

#### Scenario: Source exists outside manifest roots
- **WHEN** discovery assigns a source file to its fallback package because no manifest root is its ancestor
- **THEN** hierarchy construction uses that package and completes without a panic

