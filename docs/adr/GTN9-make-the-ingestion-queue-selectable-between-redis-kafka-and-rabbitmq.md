---
id: ADR-GTN9
type: adr
status: accepted
---
# ADR-GTN9: Make the ingestion queue selectable between Redis, Kafka and RabbitMQ

## Context

The collector must acknowledge events quickly while consumers write to ClickHouse at their own pace, and several independent consumers need every event. Operators have different infrastructure: small installs prefer Redis, large ones Kafka, some already run RabbitMQ.

## Decision

The queue is an interface in a shared Rust crate (`crates/queue`) with three implementations: Redis Streams, Kafka and RabbitMQ. The operator selects one by configuration. Each consumer reads through its own consumer group (see the consumers ADR). Background jobs (webhooks, exports, deletion) are separate from event ingestion and use Postgres-backed jobs in version 1.

A queue is required: the collector does not write directly to ClickHouse.

## Consequences

- One behavior contract for all three: at-least-once delivery, per-consumer independent reads, batch deduplication tokens.
- Replay and resume depend on retention: Kafka and Redis Streams can re-read; RabbitMQ cannot, so rebuilds use the RAW cluster.
- The same integration tests run against all three queues, which increases CI time.
- RabbitMQ needs one queue per consumer bound to a fanout exchange.
