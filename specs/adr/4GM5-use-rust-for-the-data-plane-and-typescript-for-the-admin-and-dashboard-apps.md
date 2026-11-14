---
id: ADR-4GM5
type: adr
status: accepted
---
# ADR-4GM5: Use Rust for the data plane and TypeScript for the admin and dashboard apps

## Context

Event collection, queue consumption, the public API, the MCP server and the streaming service must be fast and use little memory. The admin panel and the dashboard are CRUD-heavy user interfaces where development speed and the Node ecosystem matter more than raw throughput.

## Decision

- Rust: collector, consumer, public API, MCP server and WebSocket streaming service. Each is a separate binary built on shared crates.
- TypeScript: the admin panel (NestJS backend with its own Postgres connection and a Next.js frontend) and the dashboard (a separate NestJS backend and Next.js frontend).
- The tracker is pure TypeScript with no dependencies.

## Consequences

- Hot paths are efficient and deployable as small binaries.
- Two ecosystems to maintain; shared logic must go through contracts (`spec/`, OpenAPI), not shared code.
- Metric definitions must not be reimplemented in both languages (see the metrics ADR).
- Contributors need Rust and TypeScript skills depending on the area.
