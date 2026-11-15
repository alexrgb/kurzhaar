# schema

ClickHouse schema for the three databases (ADR XET8, ADR M898, ADR S22Y). Metrics are defined once in SQL templates; the API and dashboard databases are generated from the same files.

```
schema/
  targets.json     which templates each database gets (raw, api, dashboard) and its parameters
  templates/       SQL templates ({{database}}, {{uniq}}, {{ttl}} placeholders)
    events.sql           raw events table (30 day TTL in api and dashboard, none in raw)
    traffic_hourly.sql   views, custom events, visitors per hour
    pages_daily.sql, referrers_daily.sql, countries_daily.sql
    queries/             read queries over the rollups, shared by all readers
  migrate/         TypeScript tool that renders and applies the templates, and the golden test
```

## Metric definitions

- views: pageviews (type 0).
- custom_events: events of type 1.
- visitors: distinct session hashes among pageviews. Counted with `uniqCombined64`, so approximate for large sets (ADR S22Y). Visitors from different hours or days are merged as aggregate states, never added.
- referrers: external origins only; direct visits and internal navigation are not referrers.

## Use

Start ClickHouse (for example `deploy/compose`, minimal profile), then:

```bash
cd schema/migrate
CLICKHOUSE_URL=http://127.0.0.1:8123 node src/cli.ts migrate api
```

`render` prints the SQL without applying it. `migrate` is idempotent. It records each template with a checksum in `schema_migrations`. If a template changes after it was applied, `migrate` stops: a changed metric needs a new migration and a rollup rebuild from the RAW database.

## Tests

`pnpm test` runs the render tests. With `CLICKHOUSE_URL` set it also loads the golden dataset (`contracts/golden`) into the api and dashboard databases, compares every case with the expected values, and checks that a batch inserted twice with one deduplication token is counted once.

## Not done yet

Bounce rate and sessions (need a session-level rollup), rollup rebuild from RAW, the one-off population of the dashboard database, a deletion job for `uid_hash`, and a published image of the migration tool for compose.
