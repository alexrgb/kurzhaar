---
id: REQ-TRACKER-8PXD
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: First event immediate
The system SHALL send the first event of a page load immediately and buffer later events when batching is enabled, except for the short wait for client hints in REQ-TRACKER-EZ0J.

#### Scenario: Batching on {#S1}
- **WHEN** a page loads and the user navigates after 4 seconds
- **THEN** the first pageview is sent at once and the next event waits for the batch window or a flush
