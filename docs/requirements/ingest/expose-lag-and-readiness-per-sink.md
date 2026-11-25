---
id: REQ-INGEST-45RE
type: requirement
status: draft
links:
  decided-by:
    - ADR-HB8W
---
### Requirement: Expose lag and readiness per sink
The system SHALL serve `/healthz`, `/readyz` and `/metrics` on a listener of its own, report lag, the age of the oldest pending message, pending, inserted and dead-lettered counts per sink, and be not ready while the queue or ClickHouse cannot be reached.

#### Scenario: Metrics {#S1}
- **WHEN** the metrics are read
- **THEN** they show for the sink the lag, the pending count, the inserted events and the dead-lettered messages

#### Scenario: Queue down {#S2}
- **WHEN** the queue cannot be reached
- **THEN** `/readyz` is red

#### Scenario: ClickHouse down {#S3}
- **WHEN** ClickHouse cannot be reached
- **THEN** `/readyz` is red and `/healthz` stays green

#### Scenario: Not public {#S4}
- **WHEN** the public listener is scanned
- **THEN** it does not serve `/metrics`

#### Scenario: Age of the backlog {#S5}
- **WHEN** messages have been pending for some time
- **THEN** the metrics show the age of the oldest pending message of the sink, so a stuck sink can be alerted on

