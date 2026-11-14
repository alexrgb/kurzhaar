---
id: REQ-PRIVACY-HNMK
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: Rotating salted session hash
The system SHALL derive the session identifier from a salted hash that rotates on a schedule.

#### Scenario: After rotation {#S1}
- **WHEN** the salt rotates
- **THEN** the same visitor receives a new session identifier
