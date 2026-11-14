---
id: REQ-AUTH-SBHM
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Show API key once
The system SHALL display a newly created API key exactly once and store only its hash.

#### Scenario: Key creation {#S1}
- **WHEN** an administrator creates a key
- **THEN** the full key is shown once and later reads show only its prefix
