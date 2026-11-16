---
id: REQ-COLLECT-GZF5
type: requirement
status: draft
links:
  decided-by:
    - ADR-D3BB
---
### Requirement: Resolve the client address only through trusted proxies
The system SHALL take the client address from the connection peer unless the peer is inside a configured trusted proxy range, and only then from the right-most `X-Forwarded-For` entry that is outside the trusted ranges.

#### Scenario: No trusted proxies configured {#S1}
- **WHEN** a request carries an `X-Forwarded-For` header and no proxy ranges are configured
- **THEN** the peer address is used and the header is ignored

#### Scenario: Request through a trusted proxy {#S2}
- **WHEN** a request arrives from a trusted proxy with `X-Forwarded-For: 203.0.113.9, 10.0.0.2` and 10.0.0.0/8 is trusted
- **THEN** the client address is 203.0.113.9

#### Scenario: Address in logs {#S3}
- **WHEN** any request is handled, including one that fails
- **THEN** the address appears in no log line, metric label or queue message
