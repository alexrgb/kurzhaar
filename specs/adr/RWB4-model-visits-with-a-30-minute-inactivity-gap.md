---
id: ADR-RWB4
type: adr
status: accepted
links:
  depends-on:
    - ADR-B0B4
    - ADR-TDFQ
---
# ADR-RWB4: Model visits with a 30 minute inactivity gap

## Context

Entry page, exit page, bounce rate and visit duration need a notion of a visit. Visitors are cookieless: the session hash rotates on a schedule and cannot be used to follow a person across days. A materialized view over single events cannot decide where one visit ends and the next begins.

## Decision

- A visit is a run of events with the same session hash where no gap between consecutive events exceeds 30 minutes. A visit never spans a salt rotation.
- The consumer assigns a `visit_id` to every event before it is inserted. The id is a hash of the session hash and the timestamp of the first event of the visit, so that it is the same whenever the same events are processed in the same order.
- The consumer keeps the last seen time and visit id per session hash in memory with a 30 minute expiry. The queue is partitioned by session hash so that one consumer instance sees all events of a session. After a restart the consumer rebuilds the state from the last 30 minutes of its own cluster.
- A rollup keyed by site and visit id is maintained by a materialized view. It stores the first and last event time, entry path, exit path, number of pageviews, number of custom events and the engaged time reported by the last page.
- Definitions: a bounce is a visit with exactly one pageview and no custom events. Duration is the time between the first and last event of the visit plus the engaged time of the last page. Entry page is the path of the first pageview, exit page the path of the last.
- The RAW, API and dashboard consumers apply the same rule, and the golden dataset covers it.

## Consequences

- The consumer is no longer stateless; partitioning by session hash is required for every supported queue.
- After a crash with an incomplete rebuild a visit can be split in two. This is accepted and listed as a risk.
- Visits cannot be reconstructed for events older than the rollup; rebuilding from RAW replays events in time order with the same rule.
- Visit rollups need `visit_id` in the `events` table and a template change.
