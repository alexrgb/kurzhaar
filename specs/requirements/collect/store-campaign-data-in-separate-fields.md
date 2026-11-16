---
id: REQ-COLLECT-H4Q0
type: requirement
status: draft
links:
  decided-by:
    - ADR-B0B4
    - ADR-6RMH
---
### Requirement: Store campaign data in separate fields
The system SHALL store the UTM parameters and the click identifiers named in ADR B0B4 and ADR 6RMH (gclid, fbclid, msclkid, ttclid, li_fat_id, twclid, gad_source, dclid, wbraid, gbraid, mc_cid, igshid) as separate event fields.

#### Scenario: Campaign URL {#S1}
- **WHEN** a pageview URL contains utm_source=news and gclid=abc
- **THEN** the event has utm_source news and gclid abc in their own fields
