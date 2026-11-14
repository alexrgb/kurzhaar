---
id: ADR-KHRB
type: adr
status: accepted
---
# ADR-KHRB: Publish three separate OpenAPI contracts

## Context

The platform exposes a Rust public API (analytics, export, key validation), a NestJS admin API (users, sites, keys, webhooks) and a NestJS dashboard API. Merging them into one document couples release cycles and obscures ownership.

## Decision

Each service owns and publishes its own OpenAPI document: `public-api` (generated from Rust), `admin-api` and `dashboard-api` (generated from NestJS). The documents are collected under `spec/openapi/`. Typed clients for each are generated into `packages/api-clients`. A reverse proxy routes by path prefix so that integrators see one host, but three contracts.

## Consequences

- Each team and service changes its contract independently.
- Integrators read three documents; documentation must make clear which one to use.
- Contract tests verify that each implementation matches its document.
- The WebSocket protocol and the MCP tool list are documented separately from OpenAPI.
