---
id: REQ-COLLECT-4THR
type: requirement
status: draft
links:
  decided-by:
    - ADR-ZKD1
---
### Requirement: Detection results are cached by signal tuple
The system SHALL cache detection results by the tuple of signals so that a seen tuple needs no detector call.

#### Scenario: Repeated tuple {#S1}
- **WHEN** a thousand events arrive with the same User-Agent and signals
- **THEN** the detector is called once
