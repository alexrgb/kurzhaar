---
id: REQ-COLLECT-X2K2
type: requirement
status: draft
links:
  decided-by:
    - ADR-K8N9
---
### Requirement: Serve health and readiness separately from metrics
The system SHALL serve liveness and readiness on the public listener and Prometheus metrics only on a separate listener.

#### Scenario: Liveness {#S1}
- **WHEN** the process is running
- **THEN** `GET /healthz` answers 200 without touching the queue or Postgres

#### Scenario: Queue down {#S2}
- **WHEN** the queue is unreachable
- **THEN** `GET /readyz` answers 503 and `GET /healthz` still answers 200

#### Scenario: Metrics not public {#S3}
- **WHEN** a client asks the public listener for `/metrics`
- **THEN** the answer is 404
