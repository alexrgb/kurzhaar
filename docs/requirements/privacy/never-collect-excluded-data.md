---
id: REQ-PRIVACY-GEZF
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
---
### Requirement: Never collect excluded data
The system SHALL not collect demographics, fingerprints, mouse movement, keystrokes, form values or session replays.

#### Scenario: Code and payload inspection {#S1}
- **WHEN** the tracker and the events of a page with a form are inspected
- **THEN** no field, request or code path carries the excluded data
