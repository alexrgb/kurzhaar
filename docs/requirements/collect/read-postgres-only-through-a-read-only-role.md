---
id: REQ-COLLECT-Y61T
type: requirement
status: draft
links:
  decided-by:
    - ADR-DW71
---
### Requirement: Read Postgres only through a read-only role
The system SHALL read Postgres only through the views `collector_site_keys` and `collector_salts` as a role that can write to nothing, SHALL refuse to start when the role has write or create privileges, and SHALL read the connection string from the environment only.

#### Scenario: Role can write {#S1}
- **WHEN** the configured role can insert, update or delete in any table it can see, or create objects
- **THEN** the collector refuses to start and names the privilege

#### Scenario: Only reads {#S2}
- **WHEN** the collector runs against a Postgres that logs statements
- **THEN** it issues only `SELECT`, `LISTEN` and its own privilege checks

#### Scenario: Connection string in the file {#S3}
- **WHEN** `postgres_url` appears in the TOML file
- **THEN** the collector refuses to start and says it belongs in the environment
