# deploy

Docker Compose stack for local development and self-hosting. It currently contains the **infrastructure only** (ClickHouse, Postgres, queue). The application services (collector, consumer, public-api, mcp, stream, admin, dashboard) are not implemented yet; placeholders are commented out at the bottom of `compose/docker-compose.yml`.

Layout: `compose/` (compose file, `.env.example`, ClickHouse config), `docker/` (Dockerfiles, later), `proxy/` (reverse proxy, later).

## Quick start

```bash
cd deploy/compose
cp .env.example .env     # then change the passwords
docker compose up -d --wait
```

`COMPOSE_PROFILES` in `.env` selects what runs (default `minimal`). You can also pass `--profile` flags instead. Requires Docker Compose v2.20 or newer.

## Profiles

| Profile | Starts | Use it for |
| --- | --- | --- |
| `minimal` | 1 ClickHouse (databases `kurzhaar_raw`, `kurzhaar_api`, `kurzhaar_dashboard`), Postgres, Redis | development, small installs |
| `full` | 3 ClickHouse servers (RAW, API, dashboard; ADR XET8), Postgres. **Add one queue profile.** | production-like setups |
| `queue-redis` | Redis (Redis Streams) | queue for `full` |
| `queue-kafka` | Kafka, single-node KRaft | queue for `full` |
| `queue-rabbitmq` | RabbitMQ with management UI | queue for `full` |

```bash
COMPOSE_PROFILES=minimal docker compose up -d --wait
COMPOSE_PROFILES=full,queue-redis docker compose up -d --wait
COMPOSE_PROFILES=full,queue-kafka docker compose up -d --wait
COMPOSE_PROFILES=full,queue-rabbitmq docker compose up -d --wait
```

Use one queue profile at a time (ADR GTN9). Stop with `docker compose down`; add `-v` to also delete all data.

### ClickHouse on small hosts

ClickHouse is heavy for small installs. The `minimal` profile therefore caps the server at 50% of host RAM (`max_server_memory_usage_to_ram_ratio`), shrinks caches and background pools, limits each query to 2 GB and 2 threads (spilling to disk beyond 500 MB for GROUP BY and sort), and drops the noisy system log tables (`query_log` and `part_log` keep 7 days). In `full`, each of the three servers is capped at 30% of RAM, so the three together stay under the host limit. Edit `compose/clickhouse/*/config.xml` and `users.xml` to tune. Expect roughly 100 MB idle for the single server; real usage depends on data volume. This was not load-tested.

## Country database

The collector derives the visitor's country from an MMDB file that you provide; Kurzhaar ships none. Without one, events carry no country and the collector warns at start. Set `country_db` in the collector's TOML file or `KURZHAAR_COLLECTOR_COUNTRY_DB` to the file's path. Only the ISO country code is read, so a city database works but does not add detail.

- **DB-IP Lite Country** (recommended): free, no account, updated monthly, licence CC BY 4.0. If you publish anything derived from it you must credit "IP Geolocation by DB-IP" with a link to https://db-ip.com. Download the `.mmdb` from https://db-ip.com/db/download/ip-to-country-lite.
- **MaxMind GeoLite2 Country**: needs a free account and licence key, and its licence does not allow you to pass the file on.

The collector checks the file once a minute and starts using a replaced file without a restart. If the replacement cannot be opened, the previous database stays in use and `kurzhaar_collector_country_db_reload_failures_total` rises. A path that cannot be opened at start stops the collector. Replace the file by writing it under another name and renaming it over the old one.

## Collector and Postgres

With `KURZHAAR_COLLECTOR_POSTGRES_URL` set, the collector reads its sites and daily salts from Postgres instead of the fixed `sites` list and in-memory salts. Several collectors then share sessions. The admin backend owns the tables; the collector reads two views and writes nothing:

- `collector_site_keys (key text, site_id text)`: one row per key of every enabled site.
- `collector_salts (day date, salt bytea)`: the salts of the current and the next UTC day, 32 bytes each.

Create a role that can only read them, and nothing else (the collector refuses to start with a role that can write, create objects or is a superuser):

```sql
CREATE ROLE kurzhaar_collector LOGIN PASSWORD '...';
GRANT CONNECT ON DATABASE kurzhaar TO kurzhaar_collector;
GRANT USAGE ON SCHEMA public TO kurzhaar_collector;
GRANT SELECT ON collector_site_keys, collector_salts TO kurzhaar_collector;
```

The connection string is read from the environment only. Settings: `KURZHAAR_COLLECTOR_POSTGRES_TLS` (`true` verifies the server against the system's certificate roots), `KURZHAAR_COLLECTOR_REGISTRY_REFRESH_SECONDS` (default 60, at most 60) and `KURZHAAR_COLLECTOR_REGISTRY_GRACE_SECONDS` (default 3600). The admin backend sends `NOTIFY kurzhaar_sites` after a site or key changes, so a change takes effect within seconds; without it, within the refresh interval.

If Postgres is unreachable, the collector keeps accepting batches from the last registry it loaded. `/readyz` turns red once that registry is older than the grace period, and the collector still answers from it. A collector that has never reached Postgres answers `503` with `Retry-After`, so trackers retry instead of losing events. If today's salt is missing, the collector uses a random one in memory and counts `kurzhaar_collector_salt_fallback_total`; sessions then differ between replicas for that day.

Salts are session secrets for one day. Keep the salt table out of database dumps (for example `pg_dump --exclude-table-data=session_salts`) and do not copy the data directory of a running server to long-term storage; the software cannot remove salts from backups you keep.

## Ports

All ports bind to `127.0.0.1` by default. Set `BIND_ADDRESS=0.0.0.0` only behind a firewall.

| Service | Port(s) | Variables |
| --- | --- | --- |
| Postgres | 5432 | `POSTGRES_PORT` |
| ClickHouse (minimal) | 8123 HTTP, 9000 native | `CH_HTTP_PORT`, `CH_NATIVE_PORT` |
| ClickHouse RAW (full) | 8123, 9000 | `CH_RAW_HTTP_PORT`, `CH_RAW_NATIVE_PORT` |
| ClickHouse API (full) | 8124, 9001 | `CH_API_HTTP_PORT`, `CH_API_NATIVE_PORT` |
| ClickHouse dashboard (full) | 8125, 9002 | `CH_DASHBOARD_HTTP_PORT`, `CH_DASHBOARD_NATIVE_PORT` |
| Redis | 6379 | `REDIS_PORT` |
| Kafka (host listener) | 9094 | `KAFKA_PORT` |
| RabbitMQ | 5672 AMQP, 15672 management | `RABBITMQ_PORT`, `RABBITMQ_MANAGEMENT_PORT` |

Containers on the compose network reach each other by service name (`postgres`, `clickhouse`, `clickhouse-raw`, `clickhouse-api`, `clickhouse-dashboard`, `redis`, `kafka:9092`, `rabbitmq`).

## Volumes

Named volumes, kept across restarts: `postgres-data`, `clickhouse-data` (minimal), `clickhouse-raw-data`, `clickhouse-api-data`, `clickhouse-dashboard-data` (full), `redis-data`, `kafka-data`, `rabbitmq-data`. Compose prefixes them with the project name `kurzhaar`. The minimal and full profiles use different ClickHouse volumes, so switching profiles does not reuse data.

## Environment variables

Copy `compose/.env.example` to `compose/.env` (git-ignored). Every variable has a development default in the compose file, so the stack starts without a `.env`, but those defaults are public: **set your own passwords** before exposing any port. The example file documents each variable: image versions, credentials for Postgres, ClickHouse, Redis and RabbitMQ, ports, `REDIS_MAXMEMORY` and `KAFKA_HEAP_OPTS`.

Database schema and migrations are not applied here; the minimal profile only creates the three empty databases, and each `full` server creates its own (`kurzhaar_raw`, `kurzhaar_api`, `kurzhaar_dashboard`). Schema lives in a dedicated package (ADR M898).

## Health checks

Every service defines one, so `docker compose up --wait` returns only when all are healthy, and `docker compose ps` shows the state. Manual checks:

```bash
curl http://127.0.0.1:8123/ping                       # ClickHouse -> Ok.
docker compose exec postgres pg_isready               # Postgres
docker compose exec redis sh -c 'redis-cli -a "$REDIS_PASSWORD" ping'
```
