---
id: ADR-RH0Y
type: adr
status: proposed
links:
  refines:
    - ADR-WXH3
    - ADR-T7N1
---
# ADR-RH0Y: Retry failed sends in the tracker and date late batches by their age

## Context

The browser tracker sends each batch once. It uses `navigator.sendBeacon`, which reports only that the browser accepted the request, so the tracker never learns that the collector answered 503 (queue or Postgres down, ADR-9FWC, ADR-DW71), 429 or not at all. When the queue is down for minutes, every batch in that time is lost, although the collector deliberately answers 503 with `Retry-After` so that a client can try again.

Two properties of the system shape the answer. The collector dates events by its own receive time (REQ-COLLECT-8J26), so a batch that is sent late would be dated late. And the consumer inserts each envelope under a token that is a hash of the site key and the exact body (ADR-D3BB, ADR-HB8W), so an identical body that arrives twice is stored once.

This ADR covers the tracker in the browser. Mobile SDKs have their own storage and are decided with them. Nothing here changes what the system stores about a visitor.

## Decision

**Transport.** The tracker sends with `fetch` and `keepalive: true`, so it sees the status. `navigator.sendBeacon` stays as the fallback where `fetch` or `keepalive` is missing, and for the final flush when the page is being unloaded, where no answer can be waited for. This replaces the transport line of ADR-WXH3.

**What is retried.** A network error, a 5xx answer, 429 and 408. Any other 4xx answer (400, 404, 413) is final: the batch is dropped, because sending the same bytes again cannot succeed. A 202 is success, also for an unknown site (REQ-COLLECT-4VTR).

**How.** One batch is in flight at a time, and the batches wait in a first-in-first-out list in page memory. After a failure the sender waits before it tries again, and new batches queue behind it instead of hitting a collector that is already struggling.

- The wait is 1 s after the first failure, doubling to at most 60 s, plus a random extra of up to 50% so that many browsers do not return together.
- A `Retry-After` in seconds is honoured when it is longer than the backoff, up to 300 s.
- A success resets the wait and the next batch goes out at once.

**Limits.** At most 10 batches wait. When an eleventh arrives the oldest is dropped. A batch older than 1 hour (3,600,000 ms) is dropped. The queue lives in page memory only: a page load, a reload or a closed tab ends it. Nothing is written to `localStorage` or any other device storage (ADR-WXH3 stays in force).

**Opt-out and switches.** Before every attempt the tracker checks the opt-out switch (`lt.off`, `data-dnt`); if it is on, the queue is emptied.

**Same bytes, same token.** The body of a retried batch is the bytes of the first attempt. The batch token is therefore the same, and a batch the collector had queued while its answer was lost is not stored twice (REQ-COLLECT-X0AM, REQ-INGEST-C5GW).

**Batch age.** Because the body must not change, the age goes in the URL: `POST /v1/collect?a=<ms>`, where `a` is the number of milliseconds between the moment the batch was built and the moment this attempt is sent. The first attempt may omit it or send 0. A query parameter keeps the request a simple cross-origin request, with no preflight (REQ-COLLECT-51V3); a custom header would not.

The collector reads `a` as an integer from 0 to 3,600,000. A value that is missing, not a number or out of range is treated as 0 and never causes a rejection. It dates the batch by the receive time minus `a` (REQ-COLLECT-KSBB). `a` comes from the client, like every timestamp offset in the batch, so a client can only move its own events up to one hour into the past.

**Day salt.** The session hash uses the salt of the day of the event time. The collector keeps a day's salt for 10 minutes after the day ends (ADR-D3BB), so a late batch from the day before may find none. Then the collector uses the salt of the receive day. The session of such a batch differs from the one before midnight, as it would for any visitor who crosses midnight.

**Wire revision 1.4.** The registry `contracts/protocol/fields.json` gains an entry of a new scope, `query`, for the parameter `a`, with `since` 1.4 (ADR-VJGJ). The body format does not change, so older collectors ignore `a` and date the batch by the time it arrives, which is what they did before. Older trackers send no `a`.

**Rate limits.** Retries count against the per-address limit like any request (REQ-COLLECT-C50Y). A 429 with `Retry-After` is part of the retry rules above.

**Rejected: a persistent queue in `localStorage`.** It would recover batches after a reload or a closed tab, but it rescues only visits that return to the site, leaves events hours or days old that distort the session and the day salt, brings a consent question under ePrivacy for a project that promises no device storage, and adds cases (several tabs, duplicates, growth). It stays out of this release. It can be reconsidered as an opt-in setting if operators report real losses from long outages that the in-memory queue and a larger Redis cannot cover.

**Not done in the collector.** The collector keeps no disk spool for batches. Outages of the queue are covered by sizing Redis for the outage (ADR-HB8W) or, for large installations, by choosing Kafka (ADR-GTN9). Both are documented in `deploy/README.md`.

## Consequences

- A collector or Redis outage of a few minutes loses nothing from pages that stay open. Visitors who leave during the outage lose the batches they had not yet delivered.
- The tracker grows by the retry queue and the timer, measured against the size budget (REQ-TRACKER-YDD6 and RISK-HZNX); both transports stay, so the saving from dropping one is not available.
- A batch that is in flight when the page is unloaded cannot be retried. This is the same loss as before, no larger.
- After an outage the collector sees a burst of retries. The jitter and the single in-flight batch per page keep it short, and the rate limit remains the bound.
- Event time is correct for late batches, so the dashboard shows the right minute and the visit model of ADR-RWB4 sees events in their true order.
- The consumer needs no change: the token is the same, and ADR-HB8W already handles a delivered envelope twice.
- Revision 1.4 is a request parameter, so the shared vectors do not change; the tracker and collector tests cover it.
