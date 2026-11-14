---
id: REQ-AUTH-0D1H
type: requirement
status: draft
links:
  decided-by:
    - ADR-M1DT
---
### Requirement: Close streams of revoked keys
The system SHALL close open WebSocket connections that were authenticated with a key when the key is revoked.

#### Scenario: Open stream {#S1}
- **WHEN** a client is subscribed and its key is revoked
- **THEN** the connection is closed with an authorization error
