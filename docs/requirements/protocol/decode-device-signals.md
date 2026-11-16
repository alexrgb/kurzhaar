---
id: REQ-PROTOCOL-Y7G7
type: requirement
status: draft
links:
  decided-by:
    - ADR-ZKD1
---
### Requirement: Decode device signals
The system SHALL decode the header fields `cb` (client brands), `cm` (mobile flag), `cp` (platform), `cv` (platform major version), `tp` (maximum touch points) and `pc` (coarse primary pointer) as optional values.

#### Scenario: Chromium signals {#S1}
- **WHEN** the header contains cb::Chromium/124,Google Chrome/124|cm::0|cp::Windows|cv::15|tp::0|pc::0
- **THEN** the decoder returns two brands with their major versions, mobile false, platform Windows, platform version 15, no touch points and a fine pointer

#### Scenario: Brand with a slash {#S2}
- **WHEN** the header contains cb::Not/A)Brand/99
- **THEN** the brand is Not/A)Brand and its major version is 99

#### Scenario: Signals absent {#S3}
- **WHEN** a batch contains none of these fields
- **THEN** the decoder accepts it and returns no device signals
