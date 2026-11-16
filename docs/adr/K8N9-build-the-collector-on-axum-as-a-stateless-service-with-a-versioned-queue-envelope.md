---
id: ADR-K8N9
type: adr
status: accepted
links:
  refines:
    - ADR-4GM5
    - ADR-GTN9
---
---
# ADR-K8N9: Build the collector on axum as a stateless service with a versioned queue envelope

## Context

ADR-4GM5 puts the data plane in Rust and ADR-GTN9 puts a queue between the collector and the consumers. This record fixes how the collector itself is built: its web stack, configuration, endpoints, answers and what it passes to the queue. The collector is the one public, high-volume entry point, so it must be small, stateless and cheap to run next to a site.

## Decision

- **Stack.** Rust with `axum` on `tokio` and `tracing` for logs. The service lives in `services/collector` and uses the shared crates `protocol` (decoding), `queue` and `core`.
- **Stateless.** Replicas share nothing at run time. What a replica needs is cached in memory and refreshed: the site registry (read-only Postgres access, changes announced by `LISTEN/NOTIFY`, entries expire after at most 60 seconds, as for API keys in ADR-0GM5), the current salts (ADR-D3BB) and detection results (ADR-ZKD1). Nothing is written to disk.
- **Configuration.** One TOML file with every key overridable by an environment variable named `KURZHAAR_COLLECTOR_<KEY>`; the environment wins. The configuration is validated at start and the service refuses to start on an error. Keys: listen address, metrics listen address, trusted proxy ranges, maximum body size (default 32 KiB), maximum events per batch (default 50, the tracker sends at most 20), rate limits, country database path, salt source, detector, queue connection.
- **Endpoints.** `POST /v1/collect` takes a text/plain batch. `GET /healthz` and `GET /readyz` are on the public listener; the names follow the Kubernetes convention, are not versioned because infrastructure calls them rather than clients, and the trailing `z` keeps them clear of any route a site might proxy. `GET /metrics` (Prometheus) is on a separate listener that is not published by default.
- **Answers.** `202` with an empty body for an accepted batch, and also for a batch that is silently dropped (unknown or disabled site, bot filtered by configuration), so that the answer never reveals which keys exist. `400` for a malformed batch, `413` for a body over the limit, `429` with `Retry-After` when rate limited, `503` with `Retry-After` when the queue is unavailable. Errors carry no body that echoes the input.
- **Queue message.** One message per accepted batch, a JSON envelope with a version field: envelope version, deduplication token (ADR-D3BB), site id, receive time, country, session hash, detected device (browser, OS, device class, bot flag), tracker version, timezone offset, screen, language, identity hash, and the decoded events with tags, campaign fields and engagement or performance values. The partition key is the session hash (ADR-RWB4). Consumers ignore unknown fields and refuse an unknown major version of the envelope.
- **Order of work per request.** Check size and rate limit, decode and validate the batch, look up the site, resolve the address, detect the device, derive country, session hash and token, enqueue, then answer. The address, the User-Agent and the Client Hints are dropped when the request ends.

## Consequences

- Scaling is adding replicas behind a load balancer; no sticky routing is needed because the session hash is computed per request from shared salts.
- The collector depends on Postgres only for reading the site registry and salts; if Postgres is down, replicas keep working from the cache and `readyz` stays green while the cache is within its grace period.
- The envelope is a cross-service contract and gets its own tests; the queue crate only moves bytes and the partition key.
- JSON costs more bytes than a binary format. It is chosen for debuggability and because only Rust services read it; a binary format can replace it under a new envelope version.
- The default limits are guesses until measured (see the performance level of the test plan).
