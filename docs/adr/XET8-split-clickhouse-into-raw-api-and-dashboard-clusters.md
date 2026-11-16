---
id: ADR-XET8
type: adr
status: accepted
---
# ADR-XET8: Split ClickHouse into RAW, API and dashboard clusters

## Context

The public API and the dashboard must keep working independently of each other, and heavy dashboard queries must not slow the API. Export needs raw events; long-term history should live on cheaper storage and be the source for rebuilds.

## Decision

Three logical clusters (separate databases on one server by default, separate servers optionally):

- RAW: full append-only history of raw events. Source for rebuilding rollups and for export older than 30 days.
- API: rollups plus its own copy of raw events with a 30-day TTL, for fast export and drill-down.
- Dashboard: rollups plus its own copy of raw events with a 30-day TTL.

Each cluster is fed by its own consumer from the queue; no cluster replicates from another. The initial population of the dashboard cluster uses a one-off migration command. Exports of raw events older than 30 days are deferred (tracked as technical debt).

## Consequences

- Recent events are stored up to three times; TTL keeps the API and dashboard copies bounded.
- A failure of one cluster does not stop ingestion into the others.
- Rollups and raw events may differ for a short time, so UIs show a freshness indicator.
- Deleting personal data must run on all three clusters.
