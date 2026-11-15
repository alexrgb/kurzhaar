---
id: REQ-TRACKER-DYPM
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
---
### Requirement: Record outbound links and file downloads
The system SHALL send an event named $outbound for a click on a link to another origin and an event named $download for a click on a link to a file with a common extension.

#### Scenario: Outbound click {#S1}
- **WHEN** the user clicks a link to another origin
- **THEN** an event named $outbound with the target url is sent
