---
id: REQ-TRACKER-5AE5
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
    - ADR-9G74
---
### Requirement: Report Web Vitals
The system SHALL send one performance event for the initial page load, when the page is first hidden or unloaded, with the LCP, INP, CLS, FCP and TTFB values that the browser provides, and none for single-page navigations.

#### Scenario: Page left {#S1}
- **WHEN** a page is loaded and then left
- **THEN** one performance event with the available vitals is sent

#### Scenario: Unsupported metric {#S2}
- **WHEN** the browser does not support the Event Timing API
- **THEN** the event is sent without inp

#### Scenario: No values {#S3}
- **WHEN** the browser provides none of the vitals
- **THEN** no performance event is sent

#### Scenario: Single-page navigation {#S4}
- **WHEN** a single-page app navigates to another path
- **THEN** no performance event is sent for it
