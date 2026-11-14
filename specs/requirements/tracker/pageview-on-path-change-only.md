---
id: REQ-TRACKER-2QEW
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: Pageview on path change only
The system SHALL record a pageview only when the pathname changes.

#### Scenario: Filter change {#S1}
- **WHEN** the page updates only its query string through replaceState
- **THEN** no new pageview is recorded
