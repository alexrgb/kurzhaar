---
id: REQ-TRACKER-2P41
type: requirement
status: draft
links:
  decided-by:
    - ADR-RH0Y
---
### Requirement: Retry failed sends with backoff and limits
The system SHALL keep a batch the collector did not accept in page memory and send the same bytes again after a growing wait with jitter, honour `Retry-After`, drop a batch that is too old or no longer fits, and never write the queue to device storage.

#### Scenario: Collector answers 503 {#S1}
- **WHEN** the collector answers 503 with `Retry-After: 5` and then 202
- **THEN** the tracker sends the same body again no sooner than 5 seconds later and stops after the 202

#### Scenario: Network error {#S2}
- **WHEN** a request fails without an answer
- **THEN** the tracker retries after 1 second plus up to 50% jitter, then 2, 4 and so on to at most 60 seconds

#### Scenario: Answer that cannot succeed {#S3}
- **WHEN** the collector answers 400, 404 or 413
- **THEN** the batch is dropped and not sent again

#### Scenario: 429 {#S4}
- **WHEN** the collector answers 429 with a `Retry-After` of 30 seconds
- **THEN** the next attempt is no sooner than 30 seconds later

#### Scenario: Waiting batches in order {#S5}
- **WHEN** a batch is waiting for its retry and new batches are built
- **THEN** at most one request is in flight, the new batches wait behind it and are sent in order after a success

#### Scenario: Queue full {#S6}
- **WHEN** an eleventh batch arrives while ten are waiting
- **THEN** the oldest is dropped

#### Scenario: Too old {#S7}
- **WHEN** a batch has waited more than one hour
- **THEN** it is dropped and not sent

#### Scenario: Same bytes {#S8}
- **WHEN** a batch is sent a second time
- **THEN** its body is byte for byte the body of the first attempt

#### Scenario: No device storage {#S9}
- **WHEN** sends fail and are retried
- **THEN** nothing is written to `localStorage`, `sessionStorage`, cookies or IndexedDB

#### Scenario: Opted out {#S10}
- **WHEN** the visitor's opt-out switch is on before a retry
- **THEN** the queue is emptied and nothing is sent
