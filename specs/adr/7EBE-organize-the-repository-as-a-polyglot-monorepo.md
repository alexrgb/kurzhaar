---
id: ADR-7EBE
type: adr
status: accepted
---
# ADR-7EBE: Organize the repository as a polyglot monorepo

## Context

The project spans a TypeScript tracker, Rust services, NestJS and Next.js apps, native mobile SDKs, SQL migrations and cross-language contracts. These parts must change together (a protocol change touches the tracker, the collector and every SDK), so a single repository keeps them consistent.

## Decision

One repository, organized by artifact type:

- `apps/admin/{api,web}` and `apps/dashboard/{api,web}`: TypeScript applications.
- `services/`: Rust binaries (collector, consumer, public-api, mcp, stream).
- `crates/`: shared Rust libraries.
- `packages/`: shared TypeScript packages (tracker, protocol, api-clients).
- `sdks/`: public SDKs (react, node, react-native, flutter, ios, android).
- `schema/`: ClickHouse migrations and metric templates.
- `spec/`: cross-language contracts (protocol vectors, generated OpenAPI, golden datasets).
- `deploy/`: Docker, compose profiles and the reverse proxy.

Tooling: pnpm and Turborepo for TypeScript, a Cargo workspace for Rust, a root `Package.swift` pointing to `sdks/ios` for Swift Package Manager. Services depend on crates but not on each other; apps depend on generated API clients, never on Rust code. Languages share only `spec/`.

## Consequences

- One pull request can change a contract and all its consumers.
- CI needs path filters so a tracker change does not rebuild the Rust workspace.
- Releases use per-component tags; Swift Package Manager requires plain semver tags for the iOS SDK, which must be verified at the first release (a mirror repository is the fallback).
- Two build systems coexist; Docker and `spec/` are the only meeting points.
