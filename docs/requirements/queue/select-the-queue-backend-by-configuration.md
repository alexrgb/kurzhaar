---
id: REQ-QUEUE-BMGT
type: requirement
status: draft
links:
  decided-by:
    - ADR-9FWC
---
### Requirement: Select the queue backend by configuration
The system SHALL select the queue backend by the `queue` setting, require a connection string for every backend other than `memory`, and refuse to start on an unknown name.

#### Scenario: Redis selected {#S1}
- **WHEN** `queue` is `redis` and the URL is set
- **THEN** batches go to Redis

#### Scenario: No URL {#S2}
- **WHEN** `queue` is `redis` and no URL is set
- **THEN** the collector refuses to start and names the setting

#### Scenario: Unknown backend {#S3}
- **WHEN** `queue` is a name the collector does not know
- **THEN** the collector refuses to start and lists the known names

#### Scenario: Default {#S4}
- **WHEN** `queue` is not set
- **THEN** the in-memory queue is used and the collector warns that batches are not delivered anywhere
