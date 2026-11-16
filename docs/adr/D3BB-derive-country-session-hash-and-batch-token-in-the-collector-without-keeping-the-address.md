---
id: ADR-D3BB
type: adr
status: accepted
links:
  refines:
    - ADR-83V0
    - ADR-WXH3
---
---
# ADR-D3BB: Derive country, session hash and batch token in the collector without keeping the address

## Context

Sites are cookieless (ADR-WXH3) and the product stores no IP address (ADR-83V0). The collector still needs three things that are derived from the request: the country, a session identifier that lets visits be counted without following a person across days, and a token that makes a redelivered batch recognisable. This record fixes how they are derived and what is kept.

## Decision

- **The address lives in memory for one request.** It is read as described in REQ-COLLECT-GZF5, used for the country lookup, the session hash and the rate limiter, and never written to a log, a metric label, the queue or a database. A rate limiter keeps a bounded in-memory table keyed by address that expires idle entries.
- **Country** comes from an offline country database file in the MMDB format that the operator provides (REQ-COLLECT-82NX). Kurzhaar ships no database. Region, city and coordinates are not read. An address with no match gives an empty country.
- **Session hash** is HMAC-SHA-256 keyed with the current salt over the site id, the client address and the User-Agent header, truncated to 128 bits and written as 32 hex characters. The User-Agent is used only inside the hash. Client Hints and the tracker's signals are not part of it, so that a hint arriving late does not split a session.
- **Salt rotation.** Salts are random 32-byte values, one per UTC day. When a Postgres salt source is configured the admin backend creates tomorrow's salt in advance and deletes salts that are more than ten minutes past their day; the collector loads the current and the next salt and reads them read-only (REQ-PRIVACY-E4TN). Without a salt source the collector creates a random salt per UTC day in memory, which suits a single replica and splits sessions across replicas, and says so at start. Salts are never derived from a long-lived secret, because that would let an operator recompute old hashes.
- **Batch token.** BLAKE3 over the site key and the exact body bytes, truncated to 128 bits and written as hex (REQ-COLLECT-X0AM). The same bytes always give the same token, so the consumer can deduplicate a redelivery (RISK-F91G); the consumer derives the per-event insert token from the batch token and the event index.
- **Event time** is the receive time on the collector's clock minus the offset of the event from the last event of the batch, taken from the batch's timestamp and deltas (REQ-COLLECT-8J26). The absolute client clock is not trusted.

## Consequences

- A visit never spans a salt rotation (ADR-RWB4), and after the salt is deleted nobody, including the operator, can link yesterday's hash to a visitor. Reports in a site's local day are still cut at UTC midnight for session purposes.
- Two visitors behind one address with the same browser share a session hash; this is accepted, as in other cookieless tools.
- A batch delayed on the client (offline phone, background tab) is dated by its arrival; events are late but not lost, and the skew is limited by the batch rules (RISK-BZY4).
- An operator who keeps salts or logs addresses defeats the guarantee; the design limits this to what the software does (RISK-FP70, RISK-WWXV).
- The country accuracy and licence depend on the database the operator chooses; the documentation lists databases known to work and their terms.
