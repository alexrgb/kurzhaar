---
id: REQ-PROTOCOL-6NJT
type: requirement
status: draft
links:
  decided-by:
    - ADR-T7N1
---
### Requirement: Shared conformance vectors
The system SHALL pass the same set of encoding and decoding test vectors in every implementation language.

#### Scenario: Vectors in CI {#S1}
- **WHEN** the TypeScript, Rust, Swift and Kotlin implementations run the vectors in spec/protocol
- **THEN** all implementations produce identical results for every vector
