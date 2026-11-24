---
id: REQ-QUEUE-1PF3
type: requirement
status: draft
links:
  decided-by:
    - ADR-9FWC
---
### Requirement: Keep the order of a session across stream shards
The system SHALL choose the stream shard of a message from its partition key alone, so that messages with one key always go to one stream in the order they were written.

#### Scenario: Same key, same shard {#S1}
- **WHEN** several batches have the same session hash
- **THEN** they are in the same stream, in the order they were published

#### Scenario: Shard from the key only {#S2}
- **WHEN** two collectors publish the same key with the same shard count
- **THEN** both choose the same stream

#### Scenario: Spread {#S3}
- **WHEN** many different keys are published to several shards
- **THEN** every shard receives messages
