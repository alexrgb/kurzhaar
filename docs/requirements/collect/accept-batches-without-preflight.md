---
id: REQ-COLLECT-C50Y
type: requirement
status: draft
links:
  decided-by:
    - ADR-4GM5
---
### Requirement: Accept batches without preflight
The system SHALL accept event batches posted as text/plain to /v1/collect so that browsers send them without a CORS preflight.

#### Scenario: Beacon post {#S1}
- **WHEN** a browser sends a batch with sendBeacon
- **THEN** the collector accepts it without an OPTIONS round trip
