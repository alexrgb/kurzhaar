---
id: REQ-QUEUE-GE19
type: requirement
status: draft
links:
  decided-by:
    - ADR-9FWC
---
### Requirement: Publish accepted batches to a Redis stream
The system SHALL write each accepted batch to a Redis stream as one message with the envelope bytes and the partition key, acknowledge the batch only after Redis has replied, and answer with a retryable error when the write fails or takes longer than two seconds.

#### Scenario: Message in the stream {#S1}
- **WHEN** a batch is accepted
- **THEN** one entry with the envelope bytes in `payload` and the session hash in `key` is in the stream

#### Scenario: Redis down {#S2}
- **WHEN** Redis cannot be reached
- **THEN** the batch is not acknowledged, the answer is 503 with `Retry-After`, and nothing is stored

#### Scenario: Redis full {#S3}
- **WHEN** Redis refuses the write because its memory limit is reached
- **THEN** the answer is 503 with `Retry-After` and no earlier message is removed

#### Scenario: Slow Redis {#S4}
- **WHEN** Redis does not reply within two seconds
- **THEN** the write fails and the answer is 503 with `Retry-After`

#### Scenario: Readiness {#S5}
- **WHEN** Redis does not answer `PING` in time
- **THEN** `/readyz` is red
