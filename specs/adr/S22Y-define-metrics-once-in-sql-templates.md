---
id: ADR-S22Y
type: adr
status: accepted
---
# ADR-S22Y: Define metrics once in SQL templates

## Context

The public API (Rust) and the dashboard (TypeScript) read analytics from different clusters. If each implemented its own metric logic, numbers would diverge, the worst defect for an analytics product.

## Decision

Metric definitions (views, visitors, sessions, bounce and others) live in SQL templates in `schema/templates/`. The migration tool generates the materialized views and rollup tables for the API and dashboard clusters from the same templates. Application code only reads rollups and applies filters, sorting and pagination. A shared golden dataset in `spec/golden/` with expected results is run in CI against both clusters and must give identical numbers.

## Consequences

- One place to change a metric definition.
- Any new dimension requires a template change and a rollup rebuild from the RAW cluster.
- Unique-visitor counts use ClickHouse aggregate states and are approximate unless configured otherwise.
- Ad-hoc queries outside rollups go to raw events and are slower.
