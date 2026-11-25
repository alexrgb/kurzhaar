---
id: REQ-INGEST-XS9C
type: requirement
status: draft
links:
  decided-by:
    - ADR-HB8W
---
### Requirement: Set aside messages that cannot be read without blocking the stream
The system SHALL write a message that is not valid, has an unknown major envelope version or is rejected as bad data after five deliveries to a dead-letter stream with the reason, acknowledge it and count it, and SHALL read a newer minor version with extra fields normally.

#### Scenario: Not JSON {#S1}
- **WHEN** a message payload is not valid JSON
- **THEN** it is in the dead-letter stream with the reason and the original bytes, acknowledged, and the next message is read

#### Scenario: Unknown major version {#S2}
- **WHEN** an envelope has a major version the consumer does not know
- **THEN** it is set aside and not inserted

#### Scenario: Newer minor version {#S3}
- **WHEN** an envelope has fields the consumer does not know and a known major version
- **THEN** it is inserted and the extra fields are ignored

#### Scenario: Bad data {#S4}
- **WHEN** ClickHouse rejects an insert as bad data on the fifth delivery
- **THEN** the message is set aside and the stream moves on

#### Scenario: Counted {#S5}
- **WHEN** a message is set aside
- **THEN** the dead-letter counter of the sink rises
