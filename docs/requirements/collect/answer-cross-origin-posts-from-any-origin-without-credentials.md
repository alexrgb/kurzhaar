---
id: REQ-COLLECT-D9G1
type: requirement
status: draft
links:
  decided-by:
    - ADR-K8N9
    - ADR-WXH3
---
### Requirement: Answer cross-origin posts from any origin without credentials
The system SHALL answer cross-origin posts to `/v1/collect` with `Access-Control-Allow-Origin: *`, never allow credentials and never set a cookie.

#### Scenario: Simple post {#S1}
- **WHEN** a browser posts text/plain from any origin
- **THEN** the response allows that origin with `*` and carries no `Set-Cookie` header

#### Scenario: Preflight anyway {#S2}
- **WHEN** a client sends an OPTIONS request
- **THEN** the collector answers 204 with the allowed method POST and header Content-Type

#### Scenario: Credentials {#S3}
- **WHEN** any response is sent
- **THEN** `Access-Control-Allow-Credentials` is absent
