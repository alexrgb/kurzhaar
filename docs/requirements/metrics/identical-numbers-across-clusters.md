---
id: REQ-METRICS-9N0K
type: requirement
status: draft
links:
  decided-by:
    - ADR-S22Y
---
### Requirement: Identical numbers across clusters
The system SHALL return identical values for shared metrics from the API and dashboard clusters.

#### Scenario: Golden dataset {#S1}
- **WHEN** the golden event set is loaded into both clusters
- **THEN** every shared metric matches the expected value in both
