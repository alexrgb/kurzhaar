---
id: REQ-PROTOCOL-M4D9
type: requirement
status: draft
links:
  decided-by:
    - ADR-T7N1
---
### Requirement: Ignore unknown fields
The system SHALL ignore fields it does not recognize within a supported version.

#### Scenario: Future field {#S1}
- **WHEN** a v::1 event contains a field the decoder does not know
- **THEN** the known fields are decoded normally and no error is raised
