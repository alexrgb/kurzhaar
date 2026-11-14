---
id: REQ-AUTH-M491
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Validate a key without side effects
The system SHALL provide an endpoint that reports a key status, scopes, sites and expiry without changing any state.

#### Scenario: Check endpoint {#S1}
- **WHEN** a client calls the validation endpoint with a valid key
- **THEN** the response contains status active with its scopes and sites
