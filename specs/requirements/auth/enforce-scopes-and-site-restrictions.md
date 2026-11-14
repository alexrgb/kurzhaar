---
id: REQ-AUTH-BS03
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Enforce scopes and site restrictions
The system SHALL authorize each request against the scopes and sites bound to the key.

#### Scenario: Wrong site {#S1}
- **WHEN** a key restricted to site A requests site B
- **THEN** the request is rejected with 403
