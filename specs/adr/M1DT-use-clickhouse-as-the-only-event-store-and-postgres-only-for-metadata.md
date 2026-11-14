---
id: ADR-M1DT
type: adr
status: accepted
---
# ADR-M1DT: Use ClickHouse as the only event store and Postgres only for metadata

## Context

We first considered letting operators choose Postgres or ClickHouse for events. That doubles the test matrix and the work to implement rollups twice. Analytics queries are aggregations over large append-only data, which suits a column store. Admin data (users, roles, sites, API keys, webhooks) is relational and needs integrity constraints. MongoDB was considered and rejected: its SSPL license conflicts with an MIT open-source project and the metadata is relational.

## Decision

- ClickHouse is the only store for events and rollups in version 1.
- PostgreSQL stores admin metadata only. Rust services never write to it; they read keys and permissions and cache them in memory. Sessions are JWTs. Analytics traffic never touches Postgres.
- Rollups are built by ClickHouse materialized views, not by application code.

## Consequences

- Self-hosting always requires ClickHouse, which is heavier than Postgres for very small sites. A lightweight single-database mode may be reconsidered later.
- No Postgres-based event mode to maintain.
- Postgres load stays small and independent of event volume; revoked or changed keys must propagate to caches (see the auth requirements).
- ClickHouse is weak at updates, so writes must be idempotent through insert deduplication tokens.
