---
id: REQ-TRACKER-N7SJ
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
    - ADR-9G74
---
### Requirement: Send engagement when a page is left
The system SHALL send an engagement event with the visible time and the maximum scroll depth when a page becomes hidden, is unloaded, or is left by a single-page navigation, and restart both counters after each send.

#### Scenario: Scrolled and left {#S1}
- **WHEN** a user scrolls to 60 percent of a page and leaves after 20 seconds in focus
- **THEN** an engagement event is sent with scroll 60 and engaged time near 20000 ms

#### Scenario: Single-page navigation {#S2}
- **WHEN** a single-page app moves from /a to /b after 5 seconds
- **THEN** an engagement event for /a is sent at once and counting for /b starts from zero

#### Scenario: Tab left and revisited {#S3}
- **WHEN** the tab is hidden after 10 seconds, shown again and left after 5 more seconds
- **THEN** two engagement events are sent with engaged times near 10000 and 5000 ms

#### Scenario: Short page {#S4}
- **WHEN** the page fits in one screen
- **THEN** the scroll depth is 100
