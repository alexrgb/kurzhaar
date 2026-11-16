---
id: ADR-9G74
type: adr
status: accepted
links:
  refines:
    - ADR-B0B4
---
# ADR-9G74: Measure engagement and Web Vitals in the tracker with fixed definitions

## Context

ADR-B0B4 adds engagement events (`y::3`) and performance events (`y::4`) to the wire format, and the encoder and decoders already handle them. The tracker does not send them yet, and "engaged time", "scroll depth" and the vitals each have several common definitions. Without fixed definitions two implementations (web today, mobile SDKs later) would produce numbers that cannot be compared, and the tracker script would grow without a limit.

## Decision

**Engagement (`y::3`)**

- Engaged time (`et`) is the time the page was visible, in whole milliseconds. It does not try to detect user activity: that needs more listeners and more code, and a visible tab is the measure that Plausible and similar tools use as well.
- Scroll depth (`sc`) is the maximum of `(scroll position + viewport height) / document height`, in whole percent from 0 to 100, taken from passive scroll events and the first measurement at load. A page that fits in one screen reports 100.
- An engagement event is sent when the page becomes hidden, when the page is unloaded, and when a single-page app moves to another path (for the page that was left). After each send the counters restart, so a visitor who comes back to the tab produces another event. Engagement events of one view therefore add up: the engaged time of a view is the sum of its `et` values and its scroll depth is the largest `sc`.
- The event carries the path of the page it describes in `u`.

**Performance (`y::4`)**

- One performance event is sent for the initial page load, when the page is first hidden or unloaded, which is when LCP, CLS and INP are final enough. Single-page app navigations send none: browsers do not report vitals for soft navigations in a way that is stable across engines.
- `fcp` is the first-contentful-paint entry, `ttfb` is `responseStart` of the navigation entry, `lcp` is the last largest-contentful-paint entry, `cls` is the largest session window of layout shifts without recent input (gap under 1 s, window under 5 s), and `inp` is the longest interaction duration from the Event Timing API with a 40 ms threshold. This is a simplification of the 98th percentile that the official INP uses, and it is documented as such.
- A value the browser does not support or has not produced is left out of the event (ADR-B0B4). If no value exists, no event is sent.

**Switches.** `data-engagement="false"` turns engagement events off and `data-vitals="false"` turns performance events off. Both are on by default.

**Size budget.** The gzipped tracker with every collector on must stay at or below 4096 bytes. A test in the tracker package minifies and compresses the source and fails when the budget is exceeded, so the check runs on every pull request. Raising the budget needs an ADR amendment.

## Consequences

- Numbers are comparable between implementations once the mobile SDKs follow the same definitions.
- Visible time overstates engagement for a tab left open on a second screen; this is accepted for the smaller script.
- The metrics layer sums `et` and takes the maximum `sc` per view (ADR-S22Y).
- INP and CLS are approximations of the official values in edge cases. The vitals are for trend and comparison, not for certification against the Chrome UX Report.
- Hard-load vitals only means that a site built as a single-page app reports performance for its entry page only.
