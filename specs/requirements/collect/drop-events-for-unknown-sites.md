---
id: REQ-COLLECT-4VTR
type: requirement
status: draft
links:
  decided-by:
    - ADR-4GM5
---
### Requirement: Drop events for unknown sites
The system SHALL drop events whose site key is unknown or disabled.

#### Scenario: Unknown key {#S1}
- **WHEN** a batch carries a site key that does not exist
- **THEN** the events are not enqueued and the response does not reveal whether the key exists
