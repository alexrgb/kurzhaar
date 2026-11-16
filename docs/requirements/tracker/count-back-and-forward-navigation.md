---
id: REQ-TRACKER-33M7
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: Count back and forward navigation
The system SHALL record a pageview when the user navigates with the back or forward button to a different path.

#### Scenario: Back button {#S1}
- **WHEN** the user presses back and the pathname changes
- **THEN** one pageview is recorded for the new path
