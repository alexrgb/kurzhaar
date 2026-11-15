---
id: REQ-PROTOCOL-1EDH
type: requirement
status: draft
links:
  decided-by:
    - ADR-VJGJ
---
### Requirement: A readable field reference is generated from the registry
The system SHALL generate `docs/protocol.md` from the field registry and check in CI that the committed file matches.

#### Scenario: Reference is current {#S1}
- **WHEN** CI regenerates the reference from the registry
- **THEN** the result equals the committed `docs/protocol.md`

#### Scenario: Reference is complete {#S2}
- **WHEN** the registry lists a key
- **THEN** the reference shows the key, its name, type, unit, scope and description
