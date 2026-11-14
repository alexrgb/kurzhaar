---
id: REQ-PROTOCOL-16FY
type: requirement
status: draft
links:
  decided-by:
    - ADR-T7N1
---
### Requirement: Version first in every batch
The system SHALL start every batch with a header line whose first field is the protocol version (v::1).

#### Scenario: Header carries version {#S1}
- **WHEN** the tracker or an SDK encodes a batch
- **THEN** the first field of the first line is v::1
