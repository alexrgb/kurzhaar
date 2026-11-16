---
id: ADR-83V0
type: adr
status: accepted
---
# ADR-83V0: Delete personal data through hashed identities and audited jobs

## Context

Events are append-only and stored in three ClickHouse clusters. The right to erasure conflicts with immutability, and ClickHouse handles row deletion poorly. The product avoids direct identifiers: it stores no IP address, and sessions are rotating salted hashes. The remaining identifier is the optional `identify` id supplied by the site owner.

## Decision

- The collector stores the identify id only as `uid_hash`, an HMAC keyed with a per-site secret. The raw id is never persisted.
- `uid_hash` is part of the sorting key of the raw event tables, or covered by a bloom filter index, so deletion does not scan everything.
- Deletion is an audited job: an administrator requests it, the job removes matching events in the RAW, API and dashboard clusters, tracks completion per cluster, retries failures and records the outcome in the audit log (Postgres).
- Rollups are not rewritten. They contain aggregates and aggregate states without a recoverable identity; this limitation (removed users still count in past unique totals) is documented.
- Custom properties may contain personal data if the site owner sends it; this is the owner's responsibility and is stated in the SDK documentation.

This is not legal advice; wording should be reviewed with a lawyer before public release.

## Consequences

- Rotating the per-site secret makes old `uid_hash` values unmatchable, so rotation needs a policy.
- Deletion latency is bounded by ClickHouse mutations.
- Past unique-visitor aggregates are not corrected.
