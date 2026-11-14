---
id: REQ-PROTOCOL-SP2Z
type: requirement
status: draft
links:
  decided-by:
    - ADR-T7N1
---
### Requirement: Reject unsupported versions
The system SHALL reject a batch whose protocol version it does not support and report the version in the error.

#### Scenario: Unknown version {#S1}
- **WHEN** a decoder receives a batch with v::9
- **THEN** decoding fails with an unsupported-version error and no events are returned
