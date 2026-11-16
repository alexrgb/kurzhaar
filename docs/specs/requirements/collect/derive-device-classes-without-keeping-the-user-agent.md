---
id: REQ-COLLECT-QZRJ
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
    - ADR-ZKD1
---
### Requirement: Derive device classes without keeping the User-Agent
The system SHALL derive browser name, operating system and device class with major versions only from the User-Agent, Client Hints and the tracker's raw signals (ADR-ZKD1) and discard the raw User-Agent and signals before the event is enqueued.

#### Scenario: Event enqueued {#S1}
- **WHEN** a batch arrives with a full User-Agent string
- **THEN** the queued event holds browser, operating system and device class with major versions only and no User-Agent string
