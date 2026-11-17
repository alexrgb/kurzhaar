---
id: REQ-PRIVACY-E4TN
type: requirement
status: draft
links:
  decided-by:
    - ADR-D3BB
    - ADR-WXH3
---
### Requirement: Delete salts after rotation
The system SHALL delete a session salt no later than ten minutes after the end of its UTC day and SHALL NOT derive salts from a long-lived secret.

#### Scenario: After the day ends {#S1}
- **WHEN** ten minutes have passed since a salt's day ended
- **THEN** the salt exists in no table, cache or file

#### Scenario: New day {#S2}
- **WHEN** the first request of a new UTC day arrives
- **THEN** its session hash differs from the same visitor's hash of the day before

#### Scenario: Memory-only salt {#S3}
- **WHEN** no salt source is configured
- **THEN** the collector creates a random salt per UTC day in memory and logs once that sessions are not shared between replicas

#### Scenario: Salt creation is random {#S4}
- **WHEN** the collector creates a salt in memory
- **THEN** it is 32 bytes from the operating system's random generator, and two collectors create different salts

#### Scenario: Next day available at midnight {#S5}
- **WHEN** a request arrives in the first second of a UTC day
- **THEN** the day's salt exists and the request is not delayed or rejected

