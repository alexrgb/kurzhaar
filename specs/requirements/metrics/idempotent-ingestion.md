---
id: REQ-METRICS-63R7
type: requirement
status: draft
links:
  decided-by:
    - ADR-S22Y
---
### Requirement: Idempotent ingestion
The system SHALL insert each batch with a deduplication token so that redelivery does not change results.

#### Scenario: Redelivery {#S1}
- **WHEN** the queue delivers the same batch twice
- **THEN** rollups count the events once
