---
id: ADR-0GM5
type: adr
status: accepted
---
# ADR-0GM5: Authenticate with hashed, revocable API keys

## Context

Servers and integrations call the public API, the MCP server and the WebSocket stream. Keys are issued by the admin panel (NestJS, Postgres) and checked by Rust services on every request, so checking must not hit Postgres each time. Keys must be revocable, and revocation must take effect quickly everywhere.

## Decision

- Keys have the form `kz_live_<random>` and are shown once at creation. Only a hash (argon2id or a keyed hash suitable for high-entropy tokens) and a short prefix are stored.
- Fields: scopes (for example `read:stats`, `stream:read`), allowed sites, `created_at`, `last_used_at` (updated in batches), optional `expires_at`, `revoked_at`.
- The admin API issues and revokes keys and writes an audit entry. Rust services have read-only access, keep keys in an in-memory cache and are told about changes through Postgres `LISTEN/NOTIFY`; entries also expire after at most 60 seconds as a safety net.
- Revoking a key is permanent. Revocation closes open WebSocket connections of that key.
- Failed attempts are rate limited per source and per key prefix; hashes are compared in constant time.
- A validation endpoint reports a key's status, scopes, sites and expiry without side effects. Responses to invalid keys do not reveal why.
- Admin and dashboard users sign in with sessions issued as short-lived JWTs verified by signature.

## Consequences

- Postgres sees only a trickle of key traffic, independent of event volume.
- A leaked key can be neutralized within a minute.
- Scope semantics and key format are a cross-language contract and need shared tests.
- Temporary suspension of a key is not supported in version 1.
