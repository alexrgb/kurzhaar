---
id: ADR-M898
type: adr
status: accepted
---
# ADR-M898: Keep ClickHouse schema and migrations in a dedicated package

## Context

The ClickHouse schema (raw tables, materialized views, rollups) is a contract used by Rust services and by the TypeScript dashboard. Letting either language own it creates hidden coupling, and the same metric definitions must produce views for two clusters.

## Decision

A top-level `schema/` package holds plain SQL migrations for the `raw`, `api` and `dashboard` clusters, SQL templates for metrics (`schema/templates/`), and a small migration tool that applies them and generates the materialized views from the templates. Neither NestJS nor Rust owns the schema; both are users of it. Migrations are versioned and idempotent, and CI applies them to a fresh ClickHouse and runs the golden dataset against them.

## Consequences

- One reviewed place for every schema change.
- Services must tolerate a schema version range during rolling upgrades.
- A new dimension or metric means a template change followed by a rollup rebuild from the RAW cluster.
- The migration tool is a deployable artifact in the compose setups.
