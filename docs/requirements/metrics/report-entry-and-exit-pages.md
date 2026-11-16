---
id: REQ-METRICS-R6AA
type: requirement
status: draft
links:
  decided-by:
    - ADR-RWB4
---
### Requirement: Report entry and exit pages
The system SHALL report the path of the first pageview of a visit as its entry page and the path of the last pageview as its exit page.

#### Scenario: Three page visit {#S1}
- **WHEN** a visit views /a then /b then /c
- **THEN** the entry page is /a and the exit page is /c
