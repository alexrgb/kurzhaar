---
id: REQ-TRACKER-50ST
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
---
### Requirement: Strip query strings by default
The system SHALL remove query parameters other than utm parameters and known click identifiers before sending.

#### Scenario: URL with token {#S1}
- **WHEN** the URL contains ?token=abc&utm_source=x
- **THEN** only utm_source is sent
