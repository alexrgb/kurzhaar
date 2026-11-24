---
id: ADR-9FWC
type: adr
status: proposed
links:
  refines:
    - ADR-GTN9
    - ADR-K8N9
---
# ADR-9FWC: Write ingestion events to Redis Streams with consumer-side trimming

## Context

ADR-GTN9 makes the queue selectable and says Redis Streams is one backend. The `minimal` compose profile already runs Redis with AOF persistence, `noeviction` and a password. This record fixes the stream layout, what the collector's write guarantees, how messages are trimmed and what access each side needs, so that the collector, the consumers and the operator documentation agree. Kafka and RabbitMQ get their own records.

## Decision

- **Layout.** Messages go to `kurzhaar:events:<shard>` (the prefix is configurable). A message has two fields: `payload`, the envelope bytes of ADR-K8N9 written as they are, and `key`, the partition key (the session hash), kept for diagnostics. The number of shards is configured (`queue_shards`, default 1) and the shard of a message is the first four bytes of BLAKE3 of its key, modulo the shard count. A change of the shard count is a deployment change: it is made after the consumers have drained the streams, because it moves sessions between streams.
- **Order.** Redis keeps the order of one stream, so messages with the same key (always the same shard) keep their order, which is what the contract of `Queue::publish` promises (REQ-QUEUE-1PF3). More shards spread the load of a very busy install over more keys and, if the operator uses Redis Cluster later, over more nodes; Redis Cluster itself is not supported in version 1, only a single instance or Sentinel.
- **Write.** The collector writes with `XADD <stream> * payload <bytes> key <key>` and answers 202 only after Redis has replied (REQ-QUEUE-GE19). The write has a timeout of two seconds; a timeout, a lost connection or an error such as `OOM command not allowed` makes the publish fail, and the collector answers `503` with `Retry-After`. The connection is a managed one that reconnects by itself; readiness (`/readyz`) is a `PING` with a short timeout.
- **Durability, said plainly.** With AOF `appendfsync everysec`, which the compose file uses, Redis can lose up to about a second of acknowledged writes in a crash. An operator who cannot accept that sets `appendfsync always` and pays in throughput. The documentation says this and does not call the queue lossless.
- **No trimming by the collector.** The collector never uses `MAXLEN` and never deletes. With `maxmemory-policy noeviction`, a full Redis refuses writes, the collector answers `503` and trackers retry, which is the intended back pressure: events wait on the clients rather than being dropped inside the queue (REQ-QUEUE-YATM).
- **Trimming by consumers.** A stream grows until the consumers trim it. One retention job in the `consumer` binary (ADR-TDFQ) runs `XTRIM <stream> MINID <id>` where the id is the smallest of: the oldest pending entry and the last delivered id of every consumer group on the stream, and the id of the time `retention_hours` ago (default 24). Entries that any group has not read or not acknowledged are never removed, and entries newer than the retention period are kept for replay.
- **Groups.** Each sink (`api`, `dash`, `raw`) and the WebSocket `stream` service has its own group, created with `XGROUP CREATE <stream> <group> 0 MKSTREAM` so a new group starts from the oldest retained entry. Consumers read with `XREADGROUP`, acknowledge with `XACK` after the ClickHouse insert, and take over entries that another consumer left pending for too long with `XAUTOCLAIM`. Delivery is at least once; the batch token in the envelope makes the insert idempotent (ADR-D3BB).
- **Access.** The connection string, which carries the password, is read from the environment only (`KURZHAAR_COLLECTOR_QUEUE_URL`); `rediss://` turns on TLS. The documentation gives Redis ACL users: the collector's needs only `XADD` and `PING` on `kurzhaar:events:*`; consumers need `XREADGROUP`, `XACK`, `XAUTOCLAIM`, `XGROUP`, `XPENDING`, `XINFO` and, for the retention job, `XTRIM` (REQ-QUEUE-754R).
- **Selection.** `queue = "memory" | "redis"` in the collector's configuration (Kafka and RabbitMQ are added with their own records). `redis` needs the URL; a missing URL or an unknown name stops the start. `memory` stays the default for development and warns at start (REQ-QUEUE-BMGT).
- **Driver.** The `redis` crate with its Tokio connection manager and rustls for TLS.

## Consequences

- The `minimal` profile needs no new component for the queue.
- A consumer that stops makes Redis grow until `maxmemory`, then the collector answers 503; operators must watch consumer lag and Redis memory, and the documentation lists the metrics.
- Trimming depends on the consumer binary; a deployment with the collector only fills Redis. This is deliberate and documented.
- Changing the shard count needs a drain, which is an operator action in the release notes (ADR-WJV8).
- Redis loses replay beyond the retention period, as ADR-GTN9 says; rebuilds beyond it use the RAW cluster.
- Pending-entry handling (`XAUTOCLAIM`) belongs to the consumer work and is only specified here; this record's first implementation is the collector's write side.
