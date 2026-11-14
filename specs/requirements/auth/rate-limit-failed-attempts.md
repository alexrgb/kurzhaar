---
id: REQ-AUTH-74ZF
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Rate limit failed attempts
The system SHALL limit repeated failed key checks per source and per key prefix.

#### Scenario: Brute force {#S1}
- **WHEN** a client sends many requests with invalid keys
- **THEN** further requests are throttled
