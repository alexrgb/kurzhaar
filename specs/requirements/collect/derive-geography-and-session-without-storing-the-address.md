---
id: REQ-COLLECT-FRDS
type: requirement
status: draft
links:
  decided-by:
    - ADR-4GM5
---
### Requirement: Derive geography and session without storing the address
The system SHALL derive country and session hash from the request and discard the IP address before the event is enqueued.

#### Scenario: Event enqueued {#S1}
- **WHEN** a batch arrives from a client IP
- **THEN** the queued event contains country and session hash but no IP address
