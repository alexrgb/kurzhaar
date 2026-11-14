---
id: REQ-AUTH-TNJR
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Propagate revocation quickly
The system SHALL make a revoked key stop working in every service within 60 seconds.

#### Scenario: Cache propagation {#S1}
- **WHEN** a key is revoked while services hold it in cache
- **THEN** no service accepts the key after 60 seconds
