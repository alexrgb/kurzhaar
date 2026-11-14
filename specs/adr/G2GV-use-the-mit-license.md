---
id: ADR-G2GV
type: adr
status: accepted
---
# ADR-G2GV: Use the MIT license

## Context

Kurzhaar is a public, self-hosted analytics platform. We want the widest possible adoption and no friction for companies that embed or modify it. A hosted SaaS may be built later, in a separate private effort, and would earn from convenience rather than from license restrictions. Umami, the project we studied, is also MIT-licensed while selling a hosted service.

## Decision

All code in this repository is released under the MIT license. AGPL and BUSL-style licenses were considered and rejected.

## Consequences

- Anyone can use, modify and host the software, including competitors offering a hosted version.
- No license obstacle for corporate users; contributors need no CLA.
- Our own SaaS differentiates on operations, support and convenience, not on code access.
- Dependencies must stay compatible with MIT distribution (for example MongoDB's SSPL is avoided).
