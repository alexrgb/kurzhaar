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
- **THEN** the decoder returns the brands Chromium/124 and Google Chrome/124, mobile 0, platform Windows, platform version 15, 0 touch points and pointer 0

#### Scenario: Brand with a slash {#S2}
- **WHEN** the header contains cb::Not/A)Brand/99
- **THEN** the decoder returns one brand entry Not/A)Brand/99, whose major version is what follows the last slash

#### Scenario: Signals absent {#S3}
- **WHEN** a batch contains none of these fields
- **THEN** the decoder accepts it and returns no device signals
