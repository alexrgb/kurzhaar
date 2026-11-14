---
id: ADR-T7N1
type: adr
status: accepted
---
# ADR-T7N1: Use a compact text wire format for tracker events

## Context

Umami sends a JSON object per event with long keys and absolute timestamps. At high volume this wastes bandwidth. We need a format that is small, simple to produce in a tiny script, human-readable for debugging, and extensible without positional fields.

## Decision

Wire format version 1 is plain text, one event per line, sent as `text/plain` (no CORS preflight):

- Line 1 is the batch header: `v::1|k::<key>|s::<WxH>|l::<lang>|i::<id>`.
- Each following line is one event: fields as `key::value` joined by `|`.
- The first event carries a full timestamp `ts::<epoch ms>`; later events carry `d::<delta ms>`.
- Event type `y`: 0 pageview (omitted), 1 custom event, 2 identify. Booleans are 1 and 0.
- Short keys: `n` name, `u` url, `t` title, `r` referrer, `p.<name>` property.
- Escaping: `%`, `|`, CR and LF become `%XX`; property names also escape `:`. Fields split on the first `::`.
- The version is the first field of every batch.

A reference decoder and shared test vectors live in `spec/protocol` and are run by every implementation.

## Consequences

- Roughly four times less traffic than Umami for unbatched events (measured on prototype payloads, with header sizes assumed).
- New fields can be added without breaking older decoders; incompatible changes bump the version.
- Every language (TypeScript, Rust, Swift, Kotlin) must pass the same vectors.
- Text parsing in the collector must be strict about malformed input and size limits.
