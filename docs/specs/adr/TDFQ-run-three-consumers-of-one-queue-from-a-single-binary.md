---
id: ADR-TDFQ
type: adr
status: accepted
---
# ADR-TDFQ: Run three consumers of one queue from a single binary

## Context

Events must reach three ClickHouse clusters independently. The queue is selectable (Redis Streams, Kafka or RabbitMQ), so the design must work with all three.

## Decision

A single Rust binary, `consumer`, runs in one of three modes selected by configuration (`--sink=api|dash|raw`). Each mode uses its own consumer group (Kafka consumer group, Redis Streams group, or a dedicated queue bound to a fanout exchange in RabbitMQ), so every sink reads every message independently. Shared code handles queue access, batching and insert deduplication tokens; each mode only differs in target schema and batch parameters. The WebSocket `stream` service is a fourth independent consumer group.

## Consequences

- One artifact to build, version and deploy; three deployments with different settings.
- Lag is observable per sink.
- The queue must retain messages until all groups have read them; RabbitMQ needs one queue per consumer.
- Redelivery is possible, so each insert carries a deduplication token.
