---
id: REQ-TRACKER-5AE5
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
---
### Requirement: Report Web Vitals
The system SHALL send one performance event per page with the LCP, INP, CLS, FCP and TTFB values that are available.

#### Scenario: Page left {#S1}
- **WHEN** a page is loaded and then left
- **THEN** one performance event with the available vitals is sent
