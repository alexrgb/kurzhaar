---
id: REQ-INGEST-C5GW
type: requirement
status: draft
links:
  decided-by:
    - ADR-HB8W
---
### Requirement: Insert each envelope once under its batch token
The system SHALL insert all events of one envelope in one insert whose deduplication token is the envelope's batch token, never mixing envelopes in one token, so that a redelivered envelope adds no rows.

#### Scenario: One insert per envelope {#S1}
- **WHEN** an envelope with several events is read
- **THEN** one insert carries all its events and the batch token

#### Scenario: Redelivery {#S2}
- **WHEN** the same envelope is delivered and inserted twice, even among other messages
- **THEN** the sink holds its events once

#### Scenario: Waits for storage {#S3}
- **WHEN** the consumer uses asynchronous inserts
- **THEN** it acknowledges only after the server has stored the data

#### Scenario: Window {#S4}
- **WHEN** many other inserts happen between the first insert and a redelivery, up to the configured window
- **THEN** the redelivery is still dropped
