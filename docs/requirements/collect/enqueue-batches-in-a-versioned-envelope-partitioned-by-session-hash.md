---
id: REQ-COLLECT-SD26
type: requirement
status: draft
links:
  decided-by:
    - ADR-K8N9
---
### Requirement: Enqueue batches in a versioned envelope partitioned by session hash
The system SHALL enqueue each accepted batch as one JSON envelope with a version, the batch token, site id, receive time, country, session hash, detected device and bot flag, and the decoded events, partitioned by session hash.

#### Scenario: Accepted batch {#S1}
- **WHEN** a valid batch is accepted
- **THEN** one message is enqueued with the envelope fields and no address, User-Agent or Client Hints

#### Scenario: Partition key {#S2}
- **WHEN** two batches of one session arrive on different replicas
- **THEN** both messages carry the same partition key

#### Scenario: Newer minor version {#S3}
- **WHEN** a consumer reads an envelope with an extra field
- **THEN** it ignores the field

#### Scenario: Unknown major version {#S4}
- **WHEN** a consumer reads an envelope with an unknown major version
- **THEN** it refuses the message instead of guessing
