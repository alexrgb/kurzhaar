---
id: REQ-INGEST-J5R5
type: requirement
status: draft
links:
  decided-by:
    - ADR-HB8W
---
### Requirement: Read through one consumer group per sink and acknowledge after a durable insert
The system SHALL read the queue through a consumer group of its own for each sink, acknowledge a message only after the sink has confirmed its insert, and finish unacknowledged work after a crash.

#### Scenario: Every sink gets every message {#S1}
- **WHEN** two sinks read the same stream
- **THEN** each of them receives every message independently of the other

#### Scenario: Acknowledged after the insert {#S2}
- **WHEN** an insert has not been confirmed
- **THEN** its messages stay pending and are not acknowledged

#### Scenario: Restart {#S3}
- **WHEN** a consumer restarts with entries still pending for it
- **THEN** it reads those entries first and inserts them

#### Scenario: Dead instance {#S4}
- **WHEN** an instance leaves entries pending for more than 60 seconds
- **THEN** another instance of the sink takes them over and inserts them

#### Scenario: Several instances {#S5}
- **WHEN** several instances share a group
- **THEN** every message is delivered to exactly one of them and one session's messages stay in order
