---
id: REQ-COLLECT-EWYP
type: requirement
status: draft
links:
  decided-by:
    - ADR-DW71
---
### Requirement: Load the site registry from Postgres into memory
The system SHALL load every enabled site key from Postgres into an in-memory snapshot, look keys up in the snapshot without waiting for Postgres, reload it on a notification and at least once a minute, and replace it atomically.

#### Scenario: Known key {#S1}
- **WHEN** a batch carries a key that is in the loaded snapshot
- **THEN** it is accepted for the key's site

#### Scenario: No query on the request path {#S2}
- **WHEN** batches arrive while Postgres is slow or blocked
- **THEN** they are answered at the usual speed and no statement is sent for them

#### Scenario: New site {#S3}
- **WHEN** a site is created and a notification is sent
- **THEN** the collector accepts its key without a restart, within seconds

#### Scenario: Lost notification {#S4}
- **WHEN** a site is disabled and no notification reaches the collector
- **THEN** its key is dropped within 60 seconds

#### Scenario: Reload in progress {#S5}
- **WHEN** a reload is running while batches arrive
- **THEN** each batch sees either the old or the new snapshot and none fails
