---
id: REQ-PROTOCOL-GEE0
type: requirement
status: draft
links:
  decided-by:
    - ADR-6RMH
---
### Requirement: Decode tags, tracker version and timezone offset
The system SHALL decode the event field `g` as a list of tags, the header field `tv` as the tracker version and the header field `tz` as a signed integer number of minutes, and treat each of them as optional.

#### Scenario: Tags with an escaped comma {#S1}
- **WHEN** an event contains g::signup,paid%2Cplan
- **THEN** the decoder returns the tags signup and paid,plan

#### Scenario: Negative timezone offset {#S2}
- **WHEN** the header contains tz::-300
- **THEN** the decoder returns an offset of -300 minutes

#### Scenario: Fields absent {#S3}
- **WHEN** a batch contains none of g, tv and tz
- **THEN** the decoder accepts it and returns no tags, version or offset
