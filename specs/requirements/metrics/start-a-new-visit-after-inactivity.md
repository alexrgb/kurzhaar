---
id: REQ-METRICS-JEEQ
type: requirement
status: draft
links:
  decided-by:
    - ADR-RWB4
---
### Requirement: Start a new visit after inactivity
The system SHALL start a new visit when more than 30 minutes pass between two events of a session or when the session hash rotates.

#### Scenario: Gap over limit {#S1}
- **WHEN** two events of one session are 31 minutes apart
- **THEN** they carry different visit ids
