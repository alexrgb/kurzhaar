---
id: ADR-6RMH
type: adr
status: accepted
links:
  refines:
    - ADR-B0B4
    - ADR-T7N1
---
# ADR-6RMH: Add tags, tracker version and timezone offset in wire revision 1.2

## Context

After a comparison with what PostHog captures by default (its documentation, November 2026), four gaps remain in the first-release data set of ADR-B0B4. Custom properties (`p.<name>`) are free-form and carry their full name in every event, so they are a poor fit for simple labels that are filtered and grouped on. Nothing tells us which tracker version produced an event, which makes bugs in the field hard to trace. The visitor's local time cannot be derived from the batch. And the list of click identifiers lacks several that ad platforms add to landing URLs.

## Decision

Wire revision 1.2 (still protocol v1; older decoders ignore the new keys, ADR-T7N1) adds:

- **`g` tags (event field).** A list of short labels on an event, written as comma-separated values, with a comma inside a tag escaped as `%2C`. Limits: at most 10 tags per event, each at most 32 characters. The tracker enforces the limits; the collector drops the excess and never rejects a batch because of tags. Tags are stored as an array of low-cardinality strings and can be used to filter and group. Custom properties stay for key-value data.
- **`tv` tracker version (header field).** The version of the tracker script that built the batch, for example `1.4.2`. It is not the protocol version `v`. It is stored per event for diagnosis.
- **`tz` timezone offset (header field).** The visitor's offset from UTC in minutes, as the browser reports it for the time of the batch, for example `120` or `-300`. It supports reports by visitor local time. An offset does not identify a country, so it is not used for geography. The IANA zone name is not collected, because it is more specific and adds more to a fingerprint (RISK-SQGK).
- **More click identifiers.** The campaign fields of ADR-B0B4 gain `gad_source`, `dclid`, `wbraid`, `gbraid`, `mc_cid` and `igshid` next to the existing six. These need no new wire key: they are extracted by the collector from the page URL, and the tracker keeps them when it strips query strings (ADR-WXH3).

The keys are listed in the field registry with `since` 1.2 (ADR-VJGJ).

Not adopted from the comparison: autocapture of clicks and inputs, feature flags, person properties and rage-click capture. Feature flags and person properties belong to product analytics and conflict with a visitor model without identities; autocapture and rage clicks come close to session replay and stay out until someone decides them separately.

## Consequences

- The tracker grows by a few dozen bytes; the size is measured under RISK-HZNX.
- The `events` table gains `tags`, `tracker_version`, `tz_offset` and six click-identifier columns. Each is a template change and a rollup rebuild from RAW (ADR-S22Y).
- A tag value is free text from the site owner, so the privacy rules for custom properties apply to it as well.
- The timezone offset is one more signal that narrows a visitor down (RISK-SQGK); it is coarse, shared by millions of people, and is not combined with other signals in storage.
