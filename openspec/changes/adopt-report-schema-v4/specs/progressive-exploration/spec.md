## REMOVED Requirements

### Requirement: JSON version 1 exposes progressive links additively
**Reason**: Version 1 was retired before version 3, and version 3 is retired by this
change, so a rule about how version 1 added fields describes no shipping
contract. The progressive links it introduced — the selected scope, indexed
paths, scope finding and comparison links, scope diff counts, comparison file
ownership, and comparison direction — are required of the current machine
report by the version-4 requirements in this change.
**Migration**: Read the version-4 requirements in `report-schema-v4`, which state the
progressive links the current object exposes and the head that answers the
selected scope before them.
