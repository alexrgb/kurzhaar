---
id: REQ-QUEUE-754R
type: requirement
status: draft
links:
  decided-by:
    - ADR-9FWC
---
### Requirement: Reach Redis with a least-privilege account
The system SHALL take the Redis connection string from the environment only, support TLS, and work for the collector with an account that may run only `XADD` and `PING` on the event streams.

#### Scenario: Connection string in the file {#S1}
- **WHEN** the queue URL appears in the TOML file
- **THEN** the collector refuses to start and says it belongs in the environment

#### Scenario: Minimal account {#S2}
- **WHEN** the collector uses an account limited to `XADD` and `PING` on `kurzhaar:events:*`
- **THEN** publishing and readiness work

#### Scenario: Account too narrow {#S3}
- **WHEN** the account lacks `XADD`
- **THEN** publishing fails visibly with 503 and the connection string is not logged

#### Scenario: TLS {#S4}
- **WHEN** the URL starts with `rediss://`
- **THEN** the connection uses TLS and verifies the server certificate
