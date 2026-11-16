---
id: REQ-PRIVACY-YGHB
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: Never store IP addresses
The system SHALL not persist client IP addresses in any queue, database or log of events.

#### Scenario: Storage inspection {#S1}
- **WHEN** events are inspected in all three ClickHouse clusters
- **THEN** no field contains an IP address
