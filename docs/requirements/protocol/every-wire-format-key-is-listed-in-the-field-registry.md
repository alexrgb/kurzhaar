---
id: REQ-PROTOCOL-87M3
type: requirement
status: draft
links:
  decided-by:
    - ADR-VJGJ
---
### Requirement: Every wire format key is listed in the field registry
The system SHALL list every key of wire format v1, with its long name, scope, type, unit, requiredness, first revision and description, in `contracts/protocol/fields.json`.

#### Scenario: A new key needs a registry entry {#S1}
- **WHEN** a key that the registry does not list is written by the encoder or the tracker
- **THEN** the registry check fails and names the key

#### Scenario: Keys are not reused {#S2}
- **WHEN** a registry entry changes the meaning of an existing key
- **THEN** the registry check fails; a retired key stays listed as retired
