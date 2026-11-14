---
id: ADR-70F9
type: adr
status: accepted
---
# ADR-70F9: Stream raw events over a public WebSocket service

## Context

Integrations and dashboards want live events without polling. The collector and the REST API are not a good place for long-lived connections. We rejected sockets for the tracker itself (unreliable on unload, one connection per tab), but a server-side stream for integrations has none of those problems.

## Decision

A separate Rust service, `stream`, serves a public WebSocket endpoint. It reads the queue through its own consumer group, so it neither loads ClickHouse nor affects the other consumers.

- Version 1 streams raw events only; no live counters.
- Clients authenticate with an API key carrying the `stream:read` scope, sent in the first message (`{"op":"auth",...}`) and never in the URL, so keys stay out of logs.
- Clients subscribe to sites and may filter by event type and name; they can unsubscribe.
- No resume cursor in version 1: delivery is live and at most once.
- Browsers are not an intended client in version 1; short-lived tokens can be added later because authentication is behind an interface.
- Limits per key: connections, subscriptions and message rate. Slow clients are dropped.

## Consequences

- Events visible in the stream are limited to the fields the key's scope allows, and never contain IP addresses.
- A revoked key closes its connections.
- Several `stream` instances each need the stream of the sites they serve; scaling by site partition is a later concern.
- An API key must never be embedded in browser code; documentation states this.
