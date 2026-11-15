---
id: REQ-PROTOCOL-1XV3
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
---
### Requirement: Decode engagement and performance events
The system SHALL decode events of type 3 (engagement) and type 4 (performance) with their optional fields and ignore absent fields.

#### Scenario: Engagement event {#S1}
- **WHEN** a batch contains an event with y::3 and et::20000 and sc::60
- **THEN** the decoder returns an engagement event with 20000 ms and 60 percent
