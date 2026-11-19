---
id: ADR-WJV8
type: adr
status: accepted
links:
  refines:
    - ADR-7EBE
    - ADR-K547
---
# ADR-WJV8: Version the platform as one release and keep contract versions separate

## Context

Kurzhaar is self-hosted: an operator runs a stack of services (collector, consumer, public API, MCP, stream, admin and dashboard) and upgrades it as a whole. Sites embed the tracker and mobile apps embed SDKs, and those are upgraded on someone else's schedule. ADR-7EBE already says releases use per-component tags and reserves plain semantic version tags for the iOS SDK, because Swift Package Manager reads them. Several compatibility versions already exist (wire format revisions, the queue envelope version, `/v1` of the APIs, the schema migrations). This record says which versions exist, what each one means and how they relate.

## Decision

- **One platform version.** The server side is released as one unit with a semantic version. The git tag is `platform@X.Y.Z` (a prefixed tag, so it never collides with the plain tags SPM needs), and every service image and the migration tool are published with the same tag. Individual Rust crates and internal TypeScript packages carry no version of their own: the Cargo workspace keeps one shared value, nothing is published to crates.io, and internal npm packages stay private.
- **Before 1.0.** Versions are `0.MINOR.PATCH`; a minor release may break compatibility. Every change that needs an action from an operator (a migration, a renamed or new required setting, a removed endpoint, a changed default) is listed in the release notes under its own heading. From 1.0 the usual rules apply: a breaking change needs a major version.
- **Changelog.** `CHANGELOG.md` at the repository root lists each platform release with the headings Added, Changed, Fixed and "Operator action". Pull requests that change behaviour add their line.
- **Tracker and SDKs have their own versions.** The browser tracker is released as `tracker@X.Y.Z` and each SDK under its own tag (ADR-K547); the iOS SDK uses plain semantic version tags. They are versioned apart because they live in other people's sites and apps for years. The tracker's version travels in the batches it sends (`tracker_version`, ADR-6RMH).
- **Contract versions are separate and mean compatibility, not release.** The wire format revision (the batch format, `docs/protocol.md`), the queue envelope version (ADR-K8N9), the version in the API paths (`/v1`, ADR-KHRB) and the schema migration number (ADR-M898) each change only when their own compatibility changes, and each has its own rules:
  - A new wire revision is additive within a major revision; the collector keeps accepting every revision it has supported since 1.0. Dropping a revision is announced in the release notes one platform minor release before it happens.
  - The envelope's minor changes add fields that consumers ignore; a new major version needs the consumer to be upgraded first.
  - API paths keep `/v1` until a breaking change forces `/v2`, and both are served during a documented overlap.
  - Migrations are numbered, applied in order and idempotent; services tolerate the schema version range of a rolling upgrade (ADR-M898).
- **Upgrade order and compatibility table.** Each release lists the contract versions it speaks (wire revisions accepted, envelope version written and read, schema version, API versions) and the upgrade order. The default order is: migration tool, consumer, then collector and the other services, because the consumer must be able to read what the next collector writes.
- **Automation comes with the first image.** A release workflow (tag, images, notes) is built when the first service image exists; until then the platform version is `0.0.0` and nothing is released. The tag and notes conventions are fixed now so that the first release follows them.

## Consequences

- An operator has one number to ask about and one set of release notes to read, and cannot end up with a collector and a consumer from unrelated releases.
- Tracker, SDKs and server evolve on different clocks; the compatibility rules above are what makes that safe, so they are requirements (REQ-RELEASE-KDXJ, REQ-RELEASE-2JGH, REQ-RELEASE-47NB) and not just habit.
- The prefixed platform tag keeps SPM working, and the open check from ADR-7EBE (whether SPM tolerates prefixed tags for other components) is unchanged: if it fails, those components move to a mirror.
- Keeping crates unversioned means a crate cannot be used outside this repository by version; if one is ever published, this record is revisited.
- Writing "Operator action" lines is a discipline that reviewers have to enforce; CONTRIBUTING.md carries the rule once the first release is near.
