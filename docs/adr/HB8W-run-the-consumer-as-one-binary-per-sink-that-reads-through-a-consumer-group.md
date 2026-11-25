---
id: ADR-HB8W
type: adr
status: proposed
links:
  refines:
    - ADR-TDFQ
    - ADR-GTN9
    - ADR-9FWC
    - ADR-D3BB
    - ADR-RWB4
---
# ADR-HB8W: Run the consumer as one binary per sink that reads through a consumer group

## Context

ADR-TDFQ says one `consumer` binary runs in a mode per ClickHouse cluster (`raw`, `api`, `dash`), each reading the whole queue through its own consumer group. ADR-9FWC fixes the Redis stream layout and leaves the read side open. This record fixes how a consumer reads, acknowledges, inserts, assigns visits and fails, in terms that also fit Kafka and RabbitMQ. It also corrects one detail of ADR-D3BB after a test (below).

## Decision

- **A read trait next to `Queue`.** `crates/queue` gets a `Consumer` trait: `read(max, block)` returns deliveries (`id`, `partition_key`, `payload`, delivery count) and `ack(ids)` confirms them. Redis implements it first; Kafka and RabbitMQ implement it with their own records. The consumer service depends on the trait only.
- **One process per sink, one group per sink.** `consumer --sink raw|api|dash` (also the WebSocket `stream` service later) has its own group, so a slow or failed sink never holds back another (ADR-XET8). On Redis the group is `kz-<sink>`, created at id `0` with `MKSTREAM`, and each instance is a consumer named by host, process id and a random part. It reads all shards of the stream in one `XREADGROUP`.
- **Acknowledge after a durable insert.** A message is acknowledged only after ClickHouse has confirmed the insert. At start an instance first reads its own pending entries (id `0`), and every minute it takes over entries that another instance left pending for more than 60 seconds (`XAUTOCLAIM`), so a crashed instance's work is finished by another or by its restart.
- **One insert per envelope, under its token.** ClickHouse deduplicates per insert, not per row. A test against ClickHouse 25.8 showed: a repeated `insert_deduplication_token` drops the second insert whether it is synchronous or asynchronous (`async_insert=1`, `wait_for_async_insert=1`, `async_insert_deduplicate=1`); the same token with different rows is dropped as well, so a token must name exactly one content. Therefore the consumer inserts all events of one envelope in one insert whose token is the envelope's batch token (ADR-D3BB), never a mix of envelopes. This replaces the per-event token of ADR-D3BB, which ClickHouse cannot use. Asynchronous inserts let the server merge the many small inserts into large parts, and `wait_for_async_insert=1` makes the reply mean the data is stored. Several envelopes are inserted in parallel (default 8), then acknowledged together.
- **A wider deduplication window.** The `events` table's `non_replicated_deduplication_window = 1000` is too small for this: at high rates 1000 inserts is a few seconds. The schema change that comes with the consumer raises it (default 100000) and says so in the release notes.
- **Messages that cannot be read are set aside.** A message that is not valid JSON, has an unknown major envelope version, or is rejected by ClickHouse as bad data after five deliveries is written to `kurzhaar:dead:<sink>` with the reason and the original bytes, counted, and acknowledged, so it never blocks the stream. The dead-letter stream is capped at 10,000 entries. A newer minor version with extra fields is read normally (REQ-COLLECT-SD26).
- **Back pressure.** While ClickHouse fails or is slow, the consumer keeps messages pending, retries with a growing delay and stops reading new ones when the number of unacknowledged messages reaches a limit (default 5000). Nothing is lost: Redis holds the messages, and when Redis is full the collector answers 503 (ADR-9FWC).
- **Visits.** The consumer assigns `visit_id` as ADR-RWB4 says, in read order per session hash. A restart rebuilds the state from the sink's own cluster for the 30 minutes before the newest event stored there, not before the current time, so that the state is not empty after a long outage. An event whose time is earlier than the last event it has seen for the session (a redelivery) joins the same visit. Insert parallelism does not change visit assignment, which happens before the insert, in order.
- **What goes into a row.** The envelope's events become rows with site, time, type, name, path, title, referrer, country, session hash, identity hash, screen, language, browser, OS, device class, bot flag, tags, campaign fields, engagement and performance values and `visit_id`. The current `events` table has only the first part of that list; the schema change that adds the rest is its own pull request in `schema/` (ADR-M898) and comes before the consumer inserts. Bots are stored and flagged; rollups leave them out by default (REQ-COLLECT-X6WF).
- **Retention.** Every consumer instance also runs the retention job of ADR-9FWC once a minute; the trim is idempotent, so instances do not need to elect a leader.
- **Client and configuration.** The official `clickhouse` crate over HTTP. Settings in a TOML file with `KURZHAAR_CONSUMER_<KEY>` overrides, validated at start: sink, queue settings as for the collector (queue URL from the environment only), ClickHouse URL (environment only), batch size and wait, insert concurrency, unacknowledged limit, retention hours. The consumer has its own listener for `/healthz`, `/readyz` and `/metrics` (REQ-INGEST-45RE).
- **Least privilege.** The consumer's Redis account needs `XREADGROUP`, `XACK`, `XAUTOCLAIM`, `XGROUP`, `XPENDING`, `XINFO`, `XTRIM` and `XADD` on the dead-letter stream; its ClickHouse user can insert into `events` and read it for the visit state, nothing more.
- **Sizing Redis and watching it.** Messages stay in Redis until every group has acknowledged them, so an outage of a consumer or of ClickHouse fills Redis at the speed of the traffic. The documentation gives the rule: `maxmemory` is at least the longest outage the operator wants to survive, in seconds, times batches per second, times about 3 KB (an estimate to be measured with real traffic), plus headroom for the retention period. The compose default of 512 MB covers hours only for small sites. When Redis is full the collector answers 503; the browser tracker sends with `sendBeacon` and does not retry, so those batches are lost, while the mobile SDKs (not yet written) keep an offline queue. The consumer exports the age of its oldest pending message and its lag (REQ-INGEST-45RE), and the documentation lists alerts: Redis memory above 70% of `maxmemory`, consumer lag growing for more than ten minutes, a rising dead-letter count and `/readyz` red.
- **Order of work.** First the read trait and the Redis implementation with the retention job, then the schema change, then the `raw` sink end to end, then `api` and `dash` with their rollups.

## Consequences

- A sink can be restarted, scaled out (more instances share a group, and the partition key keeps a session's events in one stream shard) or rebuilt without touching the others.
- Duplicates are removed by ClickHouse only inside its window; a redelivery older than the window would insert twice, which the larger window makes unlikely and the golden tests check at the boundary of the batch.
- The consumer is stateful (visit state), so a restart costs a query to its cluster and a visit can be split if the rebuild is incomplete (accepted in ADR-RWB4).
- The dead-letter stream needs someone to look at it; the metric and a documented command make a growing dead-letter count visible.
- A consumer or ClickHouse outage is survived only as long as Redis has room; past that point browser traffic is lost, which is why the sizing rule and the alerts are part of the deployment documentation and not left to chance.
- Kafka and RabbitMQ need different mechanics for pending entries and retention; their records say how they meet the same `Consumer` contract.
