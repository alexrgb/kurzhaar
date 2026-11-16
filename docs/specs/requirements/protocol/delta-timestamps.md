---
id: REQ-PROTOCOL-55QJ
type: requirement
status: draft
links:
  decided-by:
    - ADR-T7N1
---
### Requirement: Delta timestamps
The system SHALL encode the first event with a full epoch-millisecond timestamp and every later event with the delta in milliseconds from the previous event.

#### Scenario: Three events in one batch {#S1}
- **WHEN** events occur at t, t+4000 and t+4500
- **THEN** the lines carry ts::t, d::4000 and d::500 and the decoder restores the absolute times
