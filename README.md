# Kurzhaar

Light, privacy-first, self-hosted web and product analytics. MIT licensed.

> Status: early. Architecture and requirements are documented (`docs/specs/`, see `docs/README.md`); the code is a scaffold plus a prototype tracker and protocol decoders.

## Layout

| Path | Contents |
|---|---|
| `apps/admin/{api,web}`, `apps/dashboard/{api,web}` | NestJS and Next.js applications (placeholders) |
| `services/` | Rust binaries: collector, consumer, public-api, mcp, stream (stubs) |
| `crates/` | Shared Rust libraries; `protocol` already decodes wire format v1 |
| `packages/` | `tracker` (browser tracker prototype), `protocol` (reference decoder) |
| `sdks/` | React, Node, React Native, Flutter, iOS, Android (placeholders) |
| `schema/`, `contracts/`, `deploy/` | ClickHouse schema, cross-language contracts, Docker (see [`deploy/README.md`](deploy/README.md) for the compose stack) |
| `docs/` | Documentation; `docs/specs/` holds the oxispec nodes: ADRs, requirements, test plan, risks |

## Develop

```bash
pnpm install
pnpm test      # tracker and protocol tests
pnpm build     # builds packages/tracker/dist/lt.js
cargo test --workspace
swift build    # iOS SDK placeholder through the root Package.swift
oxispec check  # documentation checks
```

The protocol vectors in `contracts/protocol/vectors.json` are run by both the TypeScript and the Rust decoder.
