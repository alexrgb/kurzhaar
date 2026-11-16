---
id: ADR-F28M
type: adr
status: accepted
---
# ADR-F28M: Resist ad blockers with first-party and configurable endpoints

## Context

Content blockers match well-known analytics script and endpoint names and third-party domains, which under-counts traffic. Fully evading blockers is an arms race and also raises ethical questions about circumventing user choices.

## Decision

Reduce avoidable blocking without deceptive tricks:

- The script and the ingestion endpoint are served from the site's own domain through a reverse proxy or CNAME.
- In self-hosted setups, the script name and the collect path are configurable (environment variables) and default to neutral names.
- A framework helper (for example Next.js rewrites in an `@kurzhaar` package) makes first-party proxying a one-line setup.
- A server-side tracking SDK is provided for events that must not depend on the browser.
- The tracker honors an opt-out flag and optionally Do Not Track and Global Privacy Control, so users who explicitly opt out are respected.

## Consequences

- Operators carry some proxy configuration.
- Counts are better but never complete; documentation says so.
- Public default names may be added to blocker lists; configurability lets operators rotate them.
