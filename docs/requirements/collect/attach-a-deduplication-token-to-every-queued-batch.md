---
id: REQ-COLLECT-X0AM
type: requirement
status: draft
links:
  decided-by:
    - ADR-D3BB
---
### Requirement: Attach a deduplication token to every queued batch
The system SHALL attach to every queued batch a token that is the BLAKE3 hash of the site key and the exact body bytes, truncated to 128 bits and written as hex.

#### Scenario: Same batch twice {#S1}
- **WHEN** the same bytes are posted twice for one site key
- **THEN** both queued batches carry the same token

#### Scenario: Different batches {#S2}
- **WHEN** two bodies differ by one byte
- **THEN** their tokens differ

#### Scenario: Different sites {#S3}
- **WHEN** the same body is posted under two site keys
- **THEN** their tokens differ
