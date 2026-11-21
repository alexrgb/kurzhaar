---
id: REQ-COLLECT-8ERQ
type: requirement
status: draft
links:
  decided-by:
    - ADR-DW71
---
### Requirement: Read daily salts from Postgres
The system SHALL read the salts of the current and the next UTC day from Postgres at least once a minute and when the day changes, use them for the session hash, and fall back to a random in-memory salt for a day whose salt is missing.

#### Scenario: Two replicas {#S1}
- **WHEN** two collectors read the same salts
- **THEN** one visitor's requests get the same session hash on both

#### Scenario: Day changes {#S2}
- **WHEN** a UTC day ends
- **THEN** the next day's salt, already loaded, is used from the first millisecond

#### Scenario: Salt missing {#S3}
- **WHEN** today's salt is not in Postgres
- **THEN** the collector creates a random one in memory, logs it, counts the fallback and keeps accepting batches

#### Scenario: Salt removed {#S4}
- **WHEN** a salt is deleted in Postgres
- **THEN** the collector drops it from memory at its next read and at the latest ten minutes after its day
