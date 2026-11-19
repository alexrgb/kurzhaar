---
id: REQ-RELEASE-2JGH
type: requirement
status: draft
links:
  decided-by:
    - ADR-WJV8
---
### Requirement: State compatibility and upgrade order in every release
The system SHALL state in the notes of every platform release the wire revisions accepted, the envelope version written and read, the schema version, the API versions and the order in which services are upgraded.

#### Scenario: Compatibility table {#S1}
- **WHEN** a platform release is published
- **THEN** its notes hold the table of contract versions and the upgrade order

#### Scenario: Dropping a wire revision {#S2}
- **WHEN** a release stops accepting a wire revision
- **THEN** the notes of the previous minor release announced it

#### Scenario: Consumer before collector {#S3}
- **WHEN** a new collector writes a newer minor envelope than the running consumer knows
- **THEN** the consumer ignores the extra fields, and a new major envelope is only written after the consumer has been upgraded
