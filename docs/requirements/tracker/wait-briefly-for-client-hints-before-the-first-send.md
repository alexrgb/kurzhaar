---
id: REQ-TRACKER-EZ0J
type: requirement
status: draft
links:
  decided-by:
    - ADR-ZKD1
---
### Requirement: Wait briefly for client hints before the first send
The system SHALL wait at most 150 ms for the browser's platform version before the first send of a page load, and SHALL NOT wait at all in a browser that has no `navigator.userAgentData`.

#### Scenario: Hints arrive in time {#S1}
- **WHEN** the browser answers the high-entropy request after 20 ms
- **THEN** the first batch is sent after that and its header carries `cv`

#### Scenario: Hints are slow {#S2}
- **WHEN** the browser has not answered after 150 ms
- **THEN** the first batch is sent without `cv`, and later batches carry it once it is known

#### Scenario: No client hints in the browser {#S3}
- **WHEN** `navigator.userAgentData` does not exist
- **THEN** the first event is sent at once and the header carries only `tp` and `pc`
