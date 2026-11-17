---
id: REQ-COLLECT-G8T0
type: requirement
status: draft
links:
  decided-by:
    - ADR-D3BB
---
### Requirement: Derive the session hash from the day's salt
The system SHALL derive the session hash as HMAC-SHA-256, keyed with the salt of the UTC day of the receive time, over the site id, the client address and the User-Agent header, truncated to 128 bits and written as 32 lowercase hex characters, and SHALL NOT include any other request data.

#### Scenario: Same visitor, same day {#S1}
- **WHEN** two requests with the same site, address and User-Agent arrive on one UTC day
- **THEN** their session hashes are equal

#### Scenario: Different input {#S2}
- **WHEN** the site, the address or the User-Agent differs
- **THEN** the session hashes differ

#### Scenario: Day changes {#S3}
- **WHEN** the same request is repeated after the UTC day ends
- **THEN** the session hash differs

#### Scenario: No User-Agent {#S4}
- **WHEN** the request has no User-Agent header
- **THEN** a session hash is still derived, from an empty User-Agent

#### Scenario: Hash in the envelope and the partition key {#S5}
- **WHEN** a batch is accepted
- **THEN** the envelope's session hash is the key the message is partitioned by, and it is not derived from Client Hints or tracker signals

#### Scenario: Shared salts {#S6}
- **WHEN** two replicas use the same salt source
- **THEN** one visitor's requests get the same hash and the same partition key on both
