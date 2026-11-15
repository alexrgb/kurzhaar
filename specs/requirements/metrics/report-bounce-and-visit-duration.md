---
id: REQ-METRICS-C3TZ
type: requirement
status: draft
links:
  decided-by:
    - ADR-RWB4
---
### Requirement: Report bounce and visit duration
The system SHALL count a visit with one pageview and no custom events as a bounce and compute duration as the time between its first and last event plus the engaged time of the last page.

#### Scenario: Single page visit {#S1}
- **WHEN** a visit has one pageview and no custom events
- **THEN** it counts as a bounce
