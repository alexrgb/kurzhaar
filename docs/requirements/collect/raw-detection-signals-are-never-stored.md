---
id: REQ-COLLECT-RA57
type: requirement
status: draft
links:
  decided-by:
    - ADR-ZKD1
---
### Requirement: Raw detection signals are never stored
The system SHALL discard the User-Agent, Client Hints and the tracker's raw signals after detection and enqueue only the detected classes with major versions.

#### Scenario: Queued event {#S1}
- **WHEN** an event is enqueued
- **THEN** it holds no User-Agent, client-hint or touch-point value
