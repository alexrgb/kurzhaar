---
id: REQ-PROTOCOL-9B66
type: requirement
status: draft
links:
  decided-by:
    - ADR-T7N1
---
### Requirement: Lossless escaping
The system SHALL escape the characters percent, pipe, carriage return and line feed in values and property names so that any value round-trips unchanged.

#### Scenario: Value with separators {#S1}
- **WHEN** a property value contains a pipe character and a newline
- **THEN** decoding the encoded batch returns the identical value and does not create an extra field or event
