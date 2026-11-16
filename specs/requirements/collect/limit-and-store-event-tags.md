---
id: REQ-COLLECT-J4S5
type: requirement
status: draft
links:
  decided-by:
    - ADR-6RMH
---
### Requirement: Limit and store event tags
The system SHALL keep at most 10 tags of at most 32 characters per event, drop the excess without rejecting the batch, and store the tags together with the tracker version and the timezone offset as separate event fields.

#### Scenario: Excess tags {#S1}
- **WHEN** an event arrives with 12 tags
- **THEN** the batch is accepted and the stored event has the first 10 tags

#### Scenario: Stored fields {#S2}
- **WHEN** a batch with tv::1.4.2 and tz::120 is accepted
- **THEN** its events have tracker version 1.4.2 and timezone offset 120 in their own fields
