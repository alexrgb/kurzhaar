---
id: REQ-INGEST-2FH3
type: requirement
status: draft
links:
  decided-by:
    - ADR-HB8W
---
### Requirement: Slow down when ClickHouse is slow and stop cleanly
The system SHALL keep messages pending and retry with a growing delay while ClickHouse fails, stop reading new messages when the unacknowledged limit is reached, and on a stop signal finish the inserts in flight and acknowledge them before exiting.

#### Scenario: ClickHouse down {#S1}
- **WHEN** ClickHouse cannot be reached
- **THEN** no message is acknowledged or lost and the consumer retries with a growing delay

#### Scenario: Limit reached {#S2}
- **WHEN** the number of unacknowledged messages reaches the limit
- **THEN** the consumer reads no new messages until some are acknowledged

#### Scenario: Recovery {#S3}
- **WHEN** ClickHouse returns
- **THEN** the pending messages are inserted and acknowledged without duplicates

#### Scenario: Stop signal {#S4}
- **WHEN** the consumer is asked to stop
- **THEN** it finishes the inserts in flight, acknowledges them and exits without leaving them pending

#### Scenario: Sinks are independent {#S5}
- **WHEN** one sink's ClickHouse is down
- **THEN** the other sinks keep reading and inserting
