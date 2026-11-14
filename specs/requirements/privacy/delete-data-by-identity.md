---
id: REQ-PRIVACY-0RVD
type: requirement
status: draft
links:
  decided-by:
    - ADR-WXH3
    - ADR-XET8
---
### Requirement: Delete data by identity
The system SHALL delete all events of an identified user in the RAW, API and dashboard clusters through an audited job.

#### Scenario: Deletion request {#S1}
- **WHEN** an administrator requests deletion for an identify id
- **THEN** events with the matching uid_hash are removed from all three clusters and the request is recorded in the audit log
