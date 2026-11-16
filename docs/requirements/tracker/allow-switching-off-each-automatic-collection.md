---
id: REQ-TRACKER-4EHV
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
    - ADR-9G74
---
### Requirement: Allow switching off each automatic collection
The system SHALL let each of engagement, automatic events and Web Vitals be switched off by a script attribute: data-engagement, data-auto-events and data-vitals, each set to false.

#### Scenario: Engagement disabled {#S2}
- **WHEN** the script tag has data-engagement set to false
- **THEN** no engagement event is sent

#### Scenario: Vitals disabled {#S1}
- **WHEN** the script tag has data-vitals set to false
- **THEN** no performance event is sent
