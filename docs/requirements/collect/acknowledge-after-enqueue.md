---
id: REQ-COLLECT-Y37S
type: requirement
status: draft
links:
  decided-by:
    - ADR-4GM5
---
### Requirement: Acknowledge after enqueue
The system SHALL acknowledge a batch only after all its events have been written to the queue.

#### Scenario: Queue unavailable {#S1}
- **WHEN** the queue rejects the write
- **THEN** the collector returns a retryable error and does not acknowledge the batch
