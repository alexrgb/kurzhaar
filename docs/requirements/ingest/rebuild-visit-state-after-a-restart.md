---
id: REQ-INGEST-DX9Z
type: requirement
status: draft
links:
  decided-by:
    - ADR-HB8W
---
### Requirement: Rebuild visit state after a restart
The system SHALL assign visit ids in the order of each session, rebuild the state of the 30 minutes before the newest event in the sink's own cluster at start, and give the same visit to a redelivered event that is earlier than the last event of its session.

#### Scenario: In order {#S1}
- **WHEN** events of one session arrive within 30 minutes of each other
- **THEN** they get one visit id, and after a gap of more than 30 minutes the next gets a new one

#### Scenario: Rebuilt state {#S2}
- **WHEN** the consumer restarts in the middle of a visit
- **THEN** the next event of the session continues the visit

#### Scenario: Redelivery {#S3}
- **WHEN** an event is delivered again after later events of its session were inserted
- **THEN** it gets the visit id it had before

#### Scenario: Same replay, same ids {#S4}
- **WHEN** the same events are replayed in the same order
- **THEN** every event gets the same visit id as before

#### Scenario: Salt rotation {#S5}
- **WHEN** the session hash changes at a salt rotation
- **THEN** the next event starts a new visit

#### Scenario: After a long outage {#S6}
- **WHEN** the consumer restarts hours after the newest event stored in its cluster
- **THEN** the state is rebuilt from the 30 minutes before that event, so a session that was active just before the outage can continue its visit

