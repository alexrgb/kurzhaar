---
id: REQ-QUEUE-YATM
type: requirement
status: draft
links:
  decided-by:
    - ADR-9FWC
---
### Requirement: Never trim a message a consumer group has not acknowledged
The system SHALL leave trimming to a retention job that removes only entries older than the retention period that every consumer group has read and acknowledged, and SHALL NOT trim in the collector.

#### Scenario: Collector never trims {#S1}
- **WHEN** the collector writes any number of messages
- **THEN** it issues no `XTRIM`, `XDEL` or `MAXLEN`

#### Scenario: Unacknowledged entry kept {#S2}
- **WHEN** an entry older than the retention period is still pending for a group
- **THEN** the retention job does not remove it or anything after it

#### Scenario: Lagging group {#S3}
- **WHEN** one group has not yet read an old entry
- **THEN** the entry stays

#### Scenario: Old and read {#S4}
- **WHEN** every group has acknowledged an entry that is older than the retention period
- **THEN** the job removes it
