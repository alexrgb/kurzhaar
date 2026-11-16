---
id: REQ-TRACKER-3N0V
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: Flush on page hide
The system SHALL send buffered events when the page becomes hidden or is unloaded.

#### Scenario: Tab closed {#S1}
- **WHEN** the user closes the tab with events buffered
- **THEN** the buffered events are sent with sendBeacon
