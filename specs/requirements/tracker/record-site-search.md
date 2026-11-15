---
id: REQ-TRACKER-N9BB
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
---
### Requirement: Record site search
The system SHALL send an event named $search with the search term when the page URL contains a configured search parameter.

#### Scenario: Search results page {#S1}
- **WHEN** the URL contains q=shoes and q is the configured search parameter
- **THEN** an event named $search with term shoes is sent
