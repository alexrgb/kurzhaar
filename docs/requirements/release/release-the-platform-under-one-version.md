---
id: REQ-RELEASE-KDXJ
type: requirement
status: draft
links:
  decided-by:
    - ADR-WJV8
---
### Requirement: Release the platform under one version
The system SHALL release the server side as one platform version tagged `platform@X.Y.Z`, publish every service image and the migration tool with that tag, and list in the release notes every change that needs an action from an operator.

#### Scenario: Images of one release {#S1}
- **WHEN** a platform release is published
- **THEN** every service image and the migration tool carry the same version tag

#### Scenario: Operator action {#S2}
- **WHEN** a release adds a migration, renames or adds a required setting, removes an endpoint or changes a default
- **THEN** the release notes list it under "Operator action"

#### Scenario: Tag cannot be mistaken for the iOS SDK {#S3}
- **WHEN** a platform release is tagged
- **THEN** the tag is prefixed, so Swift Package Manager does not read it as an iOS SDK version
