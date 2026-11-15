---
id: REQ-PROTOCOL-J5CJ
type: requirement
status: draft
links:
  decided-by:
    - ADR-VJGJ
---
### Requirement: Every implementation knows every key of the registry
The system SHALL make each implementation of the wire format test that it handles every key in the registry.

#### Scenario: A lagging implementation fails {#S1}
- **WHEN** the registry gets a key that the Rust decoder does not decode
- **THEN** the Rust registry test fails

#### Scenario: Unknown keys on the wire are still ignored {#S2}
- **WHEN** a batch contains a key that is not in the registry
- **THEN** decoders ignore it, as before
