---
id: REQ-COLLECT-THCX
type: requirement
status: draft
links:
  decided-by:
    - ADR-DW71
---
### Requirement: Keep collecting while Postgres is unavailable
The system SHALL keep accepting batches from the last registry snapshot for a configurable grace period when Postgres cannot be reached, keep `/readyz` green during it, and answer `503` with `Retry-After` instead of dropping events when it has no snapshot.

#### Scenario: Outage within the grace period {#S1}
- **WHEN** Postgres becomes unreachable after a snapshot was loaded
- **THEN** batches for known keys are still accepted and `/readyz` stays green

#### Scenario: Grace period over {#S2}
- **WHEN** Postgres has been unreachable longer than the grace period
- **THEN** `/readyz` is red and batches for known keys are still accepted from the stale snapshot

#### Scenario: Start without Postgres {#S3}
- **WHEN** the collector starts and no snapshot could be loaded
- **THEN** it is not ready and answers batches with `503` and `Retry-After`, and enqueues nothing

#### Scenario: Recovery {#S4}
- **WHEN** Postgres returns
- **THEN** the next reload replaces the snapshot and `/readyz` turns green
