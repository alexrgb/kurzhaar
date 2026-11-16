---
id: REQ-COLLECT-BCES
type: requirement
status: draft
links:
  decided-by:
    - ADR-ZKD1
---
### Requirement: The tracker sends the raw signals that device detection needs
The system SHALL make the tracker send the client-hint brands, mobile flag, platform, platform version, maximum touch points and primary pointer type as optional header fields, and nothing that identifies a device model.

#### Scenario: Chromium browser {#S1}
- **WHEN** the browser exposes `navigator.userAgentData`
- **THEN** the batch header carries brands, mobile flag, platform and platform version

#### Scenario: Browser without client hints {#S2}
- **WHEN** the browser exposes no `navigator.userAgentData`
- **THEN** the header carries only touch points and pointer type and the batch is accepted

#### Scenario: No model or build numbers {#S3}
- **WHEN** the tracker builds a header
- **THEN** it contains no device model, architecture, bitness or full build version
