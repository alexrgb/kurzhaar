---
id: REQ-AUTH-ZDDT
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Audit key lifecycle
The system SHALL record creation, revocation and expiry of keys with actor and time.

#### Scenario: Key revoked {#S1}
- **WHEN** an administrator revokes a key
- **THEN** the audit log contains an entry with the actor, the key prefix and the time
