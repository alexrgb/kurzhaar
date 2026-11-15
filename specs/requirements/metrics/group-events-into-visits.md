---
id: REQ-METRICS-CHAM
type: requirement
status: draft
links:
  decided-by:
    - ADR-RWB4
---
### Requirement: Group events into visits
The system SHALL give all events of one session whose gaps do not exceed 30 minutes the same visit id.

#### Scenario: Gap within limit {#S1}
- **WHEN** two events of one session are 29 minutes apart
- **THEN** both carry the same visit id
