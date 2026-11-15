---
id: REQ-TRACKER-N7SJ
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
---
### Requirement: Send engagement when a page is left
The system SHALL send one engagement event with engaged time and maximum scroll depth when a page is left.

#### Scenario: Scrolled and left {#S1}
- **WHEN** a user scrolls to 60 percent of a page and leaves after 20 seconds in focus
- **THEN** an engagement event is sent with scroll 60 and engaged time near 20000 ms
