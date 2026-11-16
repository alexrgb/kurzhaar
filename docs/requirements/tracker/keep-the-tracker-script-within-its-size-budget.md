---
id: REQ-TRACKER-YDD6
type: requirement
status: draft
links:
  decided-by:
    - ADR-9G74
---
### Requirement: Keep the tracker script within its size budget
The system SHALL fail the build when the minified and gzipped tracker with all collectors enabled is larger than 4096 bytes.

#### Scenario: Within budget {#S1}
- **WHEN** the tracker source is minified and gzipped
- **THEN** the result is at most 4096 bytes and the check passes

#### Scenario: Over budget {#S2}
- **WHEN** a change makes the gzipped tracker larger than 4096 bytes
- **THEN** the check fails and names the size and the budget
