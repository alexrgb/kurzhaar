---
id: ADR-B0B4
type: adr
status: accepted
links:
  refines:
    - ADR-T7N1
    - ADR-WXH3
---
# ADR-B0B4: Collect a defined set of visitor and engagement data

## Context

We compared what Umami, Plausible, Google Analytics 4, Matomo, PostHog and Sentry collect (vendor documentation, November 2026). Our first version records the page, referrer, country, a salted session hash, screen size, language, custom events and an identify hash. Competitors add device and browser classes, campaign and click identifiers, visit-level metrics, engagement time, automatic interaction events and performance data. Some of what the market collects (demographics, precise location, fingerprints, session replay, form values) conflicts with a privacy-first product.

## Decision

Added to the first release:

- Browser name, operating system and device class (desktop, mobile, tablet, other), derived by the collector from the User-Agent. Only major versions are kept. The raw User-Agent is discarded before the event is queued.
- Campaign data as separate fields: `utm_source`, `utm_medium`, `utm_campaign`, `utm_term`, `utm_content` and the click identifiers `gclid`, `fbclid`, `msclkid`, `ttclid`, `li_fat_id`, `twclid`. The collector extracts them from the page URL the tracker sends; the tracker keeps sending only those query parameters (ADR WXH3).
- Engagement: when a page is left, the tracker sends an engagement event with the engaged time in milliseconds and the maximum scroll depth in percent.
- Automatic events, sent as custom events with reserved names: `$outbound` (link to another origin, property `url`), `$download` (link to a file with a common extension, properties `url` and `file`) and `$search` (site search, property `term`). Each can be switched off by a script attribute.
- Web Vitals: LCP, INP, CLS, FCP and TTFB, sent as one performance event per page.
- Visit-level metrics (see ADR RWB4).

Wire format v1 gains two event types and optional fields, which older decoders ignore (ADR T7N1):

- `y::3` engagement: `et::<ms>` engaged time, `sc::<0-100>` maximum scroll percent.
- `y::4` performance: `lcp::<ms>`, `inp::<ms>`, `cls::<decimal>`, `fcp::<ms>`, `ttfb::<ms>`; absent values are omitted.

Deferred: region and city (more sensitive than country) and JavaScript error tracking. The `y` value 5 is reserved for errors so that adding them later does not change existing types.

Never collected: demographics, interests, precise location, fingerprints, mouse movement, keystrokes, form values, session replay.

## Consequences

- The `events` table and the rollups grow by these fields; each new dimension needs a template change and a rollup rebuild from RAW (ADR S22Y).
- The tracker grows. The size of the gzipped script is measured and published with every release.
- Device and browser classes are only as precise as the major version and class; this is intentional.
- The reserved `$` prefix is not available for user event names.
- Whether any of this is personal data under local law depends on the deployment; this is not legal advice.
