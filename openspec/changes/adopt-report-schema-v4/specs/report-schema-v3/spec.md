## REMOVED Requirements

### Requirement: JSON version 3 is the machine report contract
**Reason**: Version 4 replaces it as the only machine contract.
**Migration**: `report-schema-v4` restates the contract with `schema_version: 4`
and the denormalized verdict and summary head.

### Requirement: Version 3 exposes SourceRole and trust
**Reason**: Superseded by version 4.
**Migration**: Restated in `report-schema-v4` under role, trust, and identity
contracts.

### Requirement: Version 3 owns stable package identity
**Reason**: Superseded by version 4.
**Migration**: Restated in `report-schema-v4`, which also adds `manifest_name`.

### Requirement: Version 3 separates relation kind from evidence
**Reason**: Superseded by version 4.
**Migration**: Restated in `report-schema-v4` under role, trust, and identity
contracts.

### Requirement: Version 3 exposes exact history fields
**Reason**: Superseded by version 4, which adds window fields and removes
serialized similarity and ratio floats.
**Migration**: Restated in `report-schema-v4` under role, trust, and identity
contracts and under exact-value serialization.

### Requirement: Version 3 has an executable schema and exact examples
**Reason**: The version-3 schema is retired with the version.
**Migration**: `report-schema-v4` requires a checked version-4 schema and exact
examples.

### Requirement: Version 2 is retired before first release
**Reason**: Satisfied and superseded; version 2 and version 3 are both removed
by this change and version 4 is the only documented contract.
**Migration**: `report-schema-v4` requires documentation and examples to
describe version 4 only.
