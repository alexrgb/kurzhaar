---
id: ADR-DW71
type: adr
status: proposed
links:
  refines:
    - ADR-K8N9
    - ADR-D3BB
    - ADR-M1DT
---
# ADR-DW71: Read the site registry and salts from Postgres through read-only views

## Context

The collector must know which site keys are valid and, for more than one replica, share the daily salts (ADR-K8N9, ADR-D3BB). Both live in Postgres, which the admin backend owns and Rust services never write to (ADR-M1DT). Today the collector has a fixed list of sites in its configuration and random salts in memory, which splits sessions across replicas. This record fixes how the collector reads the registry and the salts, and what it does when Postgres is not there.

## Decision

- **The admin backend owns the schema.** Migrations for these tables are part of the admin API; the collector never creates or changes a table. The collector reads two views that are the contract between the two sides: `collector_site_keys` (`key`, `site_id`) holds one row per key of every site that is enabled and not deleted, and `collector_salts` (`day`, `salt`) holds the salts of the current and the next UTC day. The site key is public (it is in the tracker snippet), so it is stored and read as plain text.
- **A read-only role.** The collector connects as a role that has `SELECT` on those two views and nothing else. At start it checks its own privileges and refuses to start if the role can write to anything it can see or create objects (REQ-COLLECT-Y61T). The connection uses TLS when the operator's configuration asks for it, and the connection string is read from the environment, never from the TOML file.
- **Snapshot, not a query per request.** The registry is small (one row per site key), so the collector loads all of it into memory as one immutable snapshot that is replaced atomically. A request looks up the snapshot and never waits for Postgres (REQ-COLLECT-EWYP). An unknown or disabled key is dropped as before (REQ-COLLECT-4VTR).
- **Refresh.** The admin backend sends `NOTIFY kurzhaar_sites` after a change to sites or keys; a collector that receives it reloads at once. Every collector also reloads at least every 60 seconds, which covers a lost notification and a dropped connection. A disabled site therefore stops being accepted within a minute at the latest.
- **When Postgres is not available.** The collector keeps serving from the last snapshot for a grace period (default one hour, configurable). Within it, `/readyz` stays green. After it, `/readyz` turns red so a load balancer can drain the replica, but the replica keeps answering from the stale snapshot rather than failing visitors' beacons. A collector that starts without any snapshot answers `503` with `Retry-After` to batches and is not ready, instead of dropping events as unknown sites (REQ-COLLECT-THCX).
- **Salts.** Salts live in an unlogged table, so they are not written to the write-ahead log or replicated. The admin backend creates tomorrow's salt in advance, and a job overwrites the bytes of salts that are more than ten minutes past their day, deletes the rows and vacuums the table (REQ-PRIVACY-E4TN). The collector reads the salts of the current and the next day once a minute and also reloads when the day changes (REQ-COLLECT-8ERQ). If today's salt is missing, the collector creates a random one in memory for that day, logs it, counts `kurzhaar_collector_salt_fallback_total` and keeps collecting; this splits sessions across replicas for the affected day, which is preferable to refusing visitors.
- **Driver.** `tokio-postgres` with `rustls`: no ORM and no compile-time query checking, two small queries, and support for `LISTEN`.
- **Configuration.** `postgres_url` (environment only), `registry_refresh_seconds` (default 60, the maximum allowed), `registry_grace_seconds` (default 3600). Without `postgres_url` the collector keeps the static site list and in-memory salts, which suits development and a single replica.

## Consequences

- Several collectors share sessions as soon as they share the salt table, and a new site works within a minute without a collector restart.
- A Postgres outage does not stop collection for an hour, but a site disabled during the outage keeps being accepted until the grace period ends or Postgres returns.
- Salts in an unlogged table are lost in a Postgres crash, which then falls back to in-memory salts for that day; this loses session continuity but never leaks a salt.
- Operators who dump the database with unlogged table data, or who copy the data directory, keep the salts in their backups (RISK-FP70). The documentation tells them to exclude the table's data from dumps, and says plainly that backups are theirs to manage.
- Dead row versions can survive until vacuum; overwriting before deleting and vacuuming a table of a few rows keeps that window short, but a file-level copy of the data directory is still outside what the software can promise.
- The admin backend gains two jobs (create tomorrow's salt, clean old salts) and the views, so its work must land before the collector can be run with `postgres_url` outside tests.
- Integration tests need a real Postgres in CI.
