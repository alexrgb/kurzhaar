# golden datasets

Event sets with expected metric values. Both the API and the dashboard ClickHouse databases must reproduce them exactly (REQ-METRICS-9N0K), and a redelivered batch must change nothing (REQ-METRICS-63R7).

Each dataset is a directory:

- `events.ndjson`: raw events, one JSON object per line, in the columns of the `events` table (`schema/templates/events.sql`).
- `expected.json`: cases with a query name from `schema/templates/queries/`, its parameters and the expected rows. Expected values are computed by hand from the events, not by running the queries.

The runner is `schema/migrate/test/golden.test.ts`. It needs a ClickHouse server (`CLICKHOUSE_URL`); without one the test is skipped.

## Datasets

- `basic`: two sites, boundaries to catch off-by-one errors (hour and day edges, a session that spans two days, direct and internal referrers, custom and identify events).
