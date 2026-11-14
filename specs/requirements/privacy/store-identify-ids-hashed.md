---
id: REQ-PRIVACY-4P6B
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: Store identify ids hashed
The system SHALL store identify ids only as an HMAC keyed with a per-site secret.

#### Scenario: Identify event {#S1}
- **WHEN** a user is identified with id u1
- **THEN** the stored value is the keyed hash and not u1
