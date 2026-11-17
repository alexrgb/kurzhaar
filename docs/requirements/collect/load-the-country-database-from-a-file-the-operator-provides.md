---
id: REQ-COLLECT-82NX
type: requirement
status: draft
links:
  decided-by:
    - ADR-D3BB
---
### Requirement: Load the country database from a file the operator provides
The system SHALL derive the country from an offline MMDB database file at a configured path, ship no database, read no region or city, and pick up a replaced file without a restart.

#### Scenario: No database configured {#S1}
- **WHEN** no path is configured
- **THEN** the collector starts, logs a warning once, and events carry an empty country

#### Scenario: Unreadable database {#S2}
- **WHEN** a path is configured but the file is missing or invalid
- **THEN** the collector refuses to start and names the path

#### Scenario: File replaced {#S3}
- **WHEN** the file at the path is replaced with a valid one
- **THEN** lookups use the new file within one minute and none fail meanwhile

#### Scenario: City database {#S4}
- **WHEN** the file is a city-level database
- **THEN** only the country code is read

#### Scenario: Address not in the database {#S5}
- **WHEN** the client address has no entry in the database
- **THEN** the event carries an empty country and the batch is accepted

#### Scenario: Invalid replacement {#S6}
- **WHEN** the file at the path is replaced with one that cannot be opened
- **THEN** lookups keep using the previous database and the failure is logged and counted
