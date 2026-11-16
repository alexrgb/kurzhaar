---
id: REQ-COLLECT-8J26
type: requirement
status: draft
links:
  decided-by:
    - ADR-D3BB
---
### Requirement: Derive event time from the server clock
The system SHALL date events by the collector's receive time minus the offset between the event and the last event of the batch, and SHALL NOT use the client's absolute clock.

#### Scenario: Client clock wrong {#S1}
- **WHEN** a batch carries a first timestamp a day off the server clock
- **THEN** events are dated from the receive time

#### Scenario: Batch of three {#S2}
- **WHEN** three events carry deltas of 0, 1200 and 800 ms
- **THEN** the last event is dated at the receive time and the earlier ones 800 and 2000 ms before it

#### Scenario: Single event {#S3}
- **WHEN** a batch has one event
- **THEN** it is dated at the receive time
