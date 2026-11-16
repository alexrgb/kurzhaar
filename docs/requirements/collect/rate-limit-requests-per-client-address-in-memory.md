---
id: REQ-COLLECT-51V3
type: requirement
status: draft
links:
  decided-by:
    - ADR-K8N9
    - ADR-D3BB
---
### Requirement: Rate limit requests per client address in memory
The system SHALL limit requests per client address with an in-memory token bucket (default 300 requests per minute with a burst of 60) and answer 429 with `Retry-After` above it.

#### Scenario: Within the limit {#S1}
- **WHEN** an address sends requests below the limit
- **THEN** all are processed

#### Scenario: Above the limit {#S2}
- **WHEN** an address exceeds its bucket
- **THEN** the collector answers 429 with `Retry-After` and enqueues nothing from those requests

#### Scenario: Idle entries {#S3}
- **WHEN** an address has been idle for ten minutes
- **THEN** its bucket is removed and the table has a configured upper bound

#### Scenario: Restart {#S4}
- **WHEN** the collector restarts
- **THEN** no address is read back from disk because none was written
