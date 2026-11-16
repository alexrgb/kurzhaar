---
id: ADR-VJGJ
type: adr
status: accepted
links:
  refines:
    - ADR-T7N1
---
# ADR-VJGJ: Keep a machine-readable registry of wire format fields

## Context

Wire format v1 uses short keys (`v`, `k`, `ts`, `d`, `y`, `n`, `u`, `et`, `lcp` and more). Their meaning is described in prose in ADR-T7N1 and in a comment of the encoder, and every new field (ADR-B0B4 adds seven) has to be remembered separately by the TypeScript decoder and encoder, the Rust decoder, the tracker and future Swift, Kotlin and Dart SDKs. The format is open: third-party SDKs and integrators read it too. Nothing today tells an implementer which keys exist, what unit they use or since when.

## Decision

`contracts/protocol/fields.json` is the single registry of wire format keys. Each entry has:

- `key`: the short key as written on the wire (`et`, or `p.*` for the property family).
- `name`: the long name used in decoded form (`engagedMs`).
- `scope`: `header` or `event`, and for event fields the event types (`y` values) where it appears.
- `type` and `unit`: `string`, `int`, `decimal`, with the unit (`ms`, `percent`) where there is one.
- `required`: whether a decoder must reject a batch without it.
- `since`: the first protocol revision that has it.
- `description`: one sentence.

The registry also lists the event types (`y` values with their names) and the escaping rules by reference to ADR-T7N1.

Rules:

- A key is added to the registry in the same pull request that first uses it in any implementation. Documents (ADR or requirement) come first, as for every change.
- Keys are never reused for another meaning. A retired key stays in the registry marked `retired`.
- Each implementation has a test that reads the registry and checks that it handles every key (decoded under the long name) and that the registry has no key the implementation invented. Unknown keys on the wire are still ignored (REQ-PROTOCOL-M4D9).
- A readable reference, `docs/protocol.md`, is generated from the registry by a script and checked in CI so it cannot drift. ADR-T7N1 keeps the decisions and links to the reference instead of repeating the table.

## Consequences

- An implementer of a new SDK starts from one file and the shared vectors.
- Adding a field fails CI in every implementation that lags behind, which addresses RISK-3B2M earlier than the vectors do.
- A small amount of upkeep: the registry, the generated reference and the vectors change together.
- The tracker keeps its own inline encoder to stay small; its test checks the keys it writes against the registry.
