---
id: REQ-COLLECT-KSBB
type: requirement
status: draft
links:
  decided-by:
    - ADR-RH0Y
---
### Requirement: Date a late batch by its reported age
The system SHALL read the batch age from the `a` query parameter as whole milliseconds from 0 to 3,600,000, date the batch by the receive time minus that age, ignore a value that is missing or invalid without rejecting the batch, and take the session salt of the event day, or of the receive day when that salt is no longer held.

#### Scenario: Late batch {#S1}
- **WHEN** a batch with three events arrives with `a=60000`
- **THEN** its last event is dated 60 seconds before the receive time and the earlier events by their offsets before that

#### Scenario: Invalid age {#S2}
- **WHEN** `a` is negative, not a number, has a fraction or is above 3,600,000
- **THEN** the batch is accepted and dated as if `a` were 0

#### Scenario: No age {#S3}
- **WHEN** the request has no `a`
- **THEN** the batch is dated by the receive time, as before

#### Scenario: Across midnight {#S4}
- **WHEN** a batch arrives 12 minutes after midnight UTC with an age that puts it before midnight
- **THEN** the event time is the earlier one and the session hash uses the salt of the receive day

#### Scenario: Same token {#S5}
- **WHEN** the same body arrives with different values of `a`
- **THEN** the batch token is the same
