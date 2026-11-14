---
id: REQ-AUTH-3FX4
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Optional expiry
The system SHALL reject a key after its expiry time when one is set.

#### Scenario: Expired key {#S1}
- **WHEN** a key with an expiry in the past is used
- **THEN** the request is rejected with 401
