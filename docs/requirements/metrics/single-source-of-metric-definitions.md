---
id: REQ-METRICS-FNX4
type: requirement
status: draft
links:
  decided-by:
    - ADR-S22Y
---
### Requirement: Single source of metric definitions
The system SHALL generate the rollups of the API and dashboard clusters from the same SQL templates.

#### Scenario: Template change {#S1}
- **WHEN** a metric definition changes in schema/templates
- **THEN** both clusters are regenerated from that one definition
