---
id: ADR-PA3H
type: adr
status: accepted
---
# ADR-PA3H: Expose analytics to agents through a read-only MCP server

## Context

AI agents increasingly query analytics. The Model Context Protocol is the established way to give them tools, but each tool definition costs context tokens, and giving agents write access to administration multiplies risk. Published comparisons suggest direct APIs or CLIs are cheaper per call, while MCP offers self-description and shared authentication; most figures come from vendors and are not independent.

## Decision

A separate Rust service, `mcp`, exposes a small set of read-only tools (for example list sites, statistics for a period, top pages, events, sources, conversions) on top of the same query layer as the public API. It authenticates with the same API keys and scopes. It does not manage sites, keys or webhooks in version 1.

Tool descriptions are short and results are compact aggregates with limits, not raw rows.

## Consequences

- The REST API remains the primary interface; agents that can call it directly lose nothing.
- Numbers from MCP, the API and the dashboard come from the same rollups.
- Administrative actions through agents may be added later with stricter authorization.
- A small tool set keeps context cost low.
