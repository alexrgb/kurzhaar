---
id: REQ-COLLECT-X6WF
type: requirement
status: draft
links:
  decided-by:
    - ADR-ZKD1
---
### Requirement: Browser, OS and device are detected by a replaceable detector in the collector
The system SHALL detect browser, operating system, device class and bot status in the collector through a detector interface, preferring client-hint signals over the User-Agent.

#### Scenario: Windows 11 {#S1}
- **WHEN** a batch carries platform "Windows" and platform version 13 or higher
- **THEN** the event is classified as Windows 11

#### Scenario: iPad in desktop mode {#S2}
- **WHEN** the User-Agent says Macintosh, touch points are more than one and the primary pointer is coarse
- **THEN** the device class is tablet

#### Scenario: Detector swapped {#S3}
- **WHEN** the operator configures another detector implementation
- **THEN** the collector uses it without changes to the tracker or the queue format

#### Scenario: Bot {#S4}
- **WHEN** the detector flags a request as a bot
- **THEN** the event is excluded from metrics by default
