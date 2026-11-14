---
id: ADR-WXH3
type: adr
status: accepted
---
# ADR-WXH3: Make the tracker cookieless with optional batching

## Context

Browsers lose requests on page unload, and many visits are one page long. Umami keeps a session token in a JavaScript variable that disappears on reload and is only an optimization. We want no client-side storage by default so that privacy law obligations stay minimal, and fewer requests where that is safe.

## Decision

- No cookies and no `localStorage` or `sessionStorage` for identification by default. Sessions are derived on the server from a salted hash that rotates (cookieless mode). An optional `data-session="tab"` mode may be added later and documented as using device storage.
- Batching is on by default and can be disabled (`data-batch="false"`): the first event of a page load is sent immediately, later events wait up to 3000 ms (configurable), up to 20 events per batch, and the queue is flushed on `visibilitychange` to hidden and on `pagehide`.
- Transport is `navigator.sendBeacon` with a `fetch` `keepalive` fallback.
- A pageview is recorded only when the pathname changes; query strings are stripped except `utm_*` and click identifiers (`data-query` = all or none overrides).
- Opt-out: `localStorage` key `lt.off`, optional `data-dnt`.
- IP addresses are used for hashing and geolocation and are never stored.

## Consequences

- No consent banner is automatically implied by the tracker's own storage, but this is not legal advice; deployments must still assess their legal basis.
- Query-only URL changes (filters, tabs) no longer produce duplicate pageviews, unlike Umami.
- Back-button navigation is counted via `popstate`.
- Without client IDs, unique-visitor counts depend on the rotating hash and are approximate across days.
