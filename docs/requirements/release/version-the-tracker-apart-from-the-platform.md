---
id: REQ-RELEASE-47NB
type: requirement
status: draft
links:
  decided-by:
    - ADR-WJV8
---
### Requirement: Version the tracker apart from the platform
The system SHALL release the browser tracker and each SDK under their own versions and tags, send the tracker version with every batch, and keep accepting every wire revision supported since 1.0.

#### Scenario: Older tracker {#S1}
- **WHEN** a site keeps an older tracker after the platform is upgraded
- **THEN** its batches are still accepted and decoded

#### Scenario: Version reported {#S2}
- **WHEN** a batch is accepted
- **THEN** the tracker version it carried is in the queue envelope
