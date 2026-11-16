---
id: REQ-TRACKER-KE2V
type: requirement
status: draft
links:
  decided-by:
    - ADR-6RMH
---
### Requirement: Send event tags, the tracker version and the timezone offset
The system SHALL let the site owner attach tags to a custom event, and SHALL send the tracker version and the visitor's timezone offset in the header of every batch.

#### Scenario: Tagged event {#S1}
- **WHEN** the site calls track with the event name signup and the tags paid and trial
- **THEN** the event carries g::paid,trial

#### Scenario: Header values {#S2}
- **WHEN** the tracker builds a batch
- **THEN** the header carries tv with the script version and tz with the offset in minutes, and no IANA zone name

#### Scenario: Too many tags {#S3}
- **WHEN** the site passes 12 tags or a tag longer than 32 characters
- **THEN** the tracker sends the first 10 tags, each cut to 32 characters
