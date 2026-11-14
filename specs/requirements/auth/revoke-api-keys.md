---
id: REQ-AUTH-B798
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Revoke API keys
The system SHALL let an administrator revoke an API key permanently.

#### Scenario: Revoked key used {#S1}
- **WHEN** a revoked key is used on any API
- **THEN** the request is rejected with 401
