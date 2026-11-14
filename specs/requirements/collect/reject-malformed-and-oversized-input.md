---
id: REQ-COLLECT-FWT0
type: requirement
status: draft
links:
  decided-by:
    - ADR-4GM5
---
### Requirement: Reject malformed and oversized input
The system SHALL enforce a maximum body size and a maximum number of events per batch and reject malformed lines.

#### Scenario: Oversized body {#S1}
- **WHEN** a batch exceeds the configured size limit
- **THEN** the collector returns an error and enqueues nothing
