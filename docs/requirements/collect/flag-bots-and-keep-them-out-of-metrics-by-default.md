---
id: REQ-COLLECT-1AYR
type: requirement
status: draft
links:
  decided-by:
    - ADR-1X7Z
    - ADR-ZKD1
---
### Requirement: Flag bots and keep them out of metrics by default
The system SHALL set a bot flag on events whose signals the detector classifies as a bot, enqueue them with the flag and never report them as device class `other`.

#### Scenario: Known crawler {#S1}
- **WHEN** a batch arrives with a crawler User-Agent
- **THEN** the envelope carries bot true and a device class other than `other`

#### Scenario: Headless browser {#S2}
- **WHEN** a batch arrives with a headless browser User-Agent
- **THEN** the envelope carries bot true

#### Scenario: Ordinary browser {#S3}
- **WHEN** a batch arrives with an ordinary browser User-Agent
- **THEN** the envelope carries bot false
