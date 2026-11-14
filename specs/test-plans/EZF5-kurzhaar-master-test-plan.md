---
id: TP-EZF5
type: test-plan
status: draft
owner: alexrgb
date: 2026-11-14
links:
  covers:
    - REQ-AUTH-ZDDT
    - REQ-AUTH-0D1H
    - REQ-AUTH-BS03
    - REQ-AUTH-3FX4
    - REQ-AUTH-TNJR
    - REQ-AUTH-74ZF
    - REQ-AUTH-B798
    - REQ-AUTH-SBHM
    - REQ-AUTH-M491
    - REQ-COLLECT-C50Y
    - REQ-COLLECT-Y37S
    - REQ-COLLECT-FRDS
    - REQ-COLLECT-4VTR
    - REQ-COLLECT-FWT0
    - REQ-METRICS-63R7
    - REQ-METRICS-9N0K
    - REQ-METRICS-FNX4
    - REQ-PRIVACY-0RVD
    - REQ-PRIVACY-YGHB
    - REQ-PRIVACY-HNMK
    - REQ-PRIVACY-4P6B
    - REQ-PROTOCOL-55QJ
    - REQ-PROTOCOL-M4D9
    - REQ-PROTOCOL-9B66
    - REQ-PROTOCOL-SP2Z
    - REQ-PROTOCOL-6NJT
    - REQ-PROTOCOL-16FY
    - REQ-TRACKER-33M7
    - REQ-TRACKER-8PXD
    - REQ-TRACKER-3N0V
    - REQ-TRACKER-2QEW
    - REQ-TRACKER-50ST
    - RISK-3B2M
    - RISK-6DCB
    - RISK-88FF
    - RISK-AMDJ
    - RISK-F91G
    - RISK-KA8M
    - RISK-QETX
    - RISK-WWXV
---
# TP-EZF5: Kurzhaar master test plan

## Scope

In scope for the first documented version: the wire protocol, the tracker, the collector, API key authentication, privacy guarantees and metric consistency between the API and dashboard clusters. Not yet covered: the admin and dashboard user interfaces, the WebSocket service, the MCP server, mobile SDKs and export of events older than 30 days (see the technical debt entry). Plans for those areas are added when their requirements are written.

## Approach

Every requirement scenario must be verified by at least one automated test that carries a `verifies:` comment pointing at the scenario. Cross-language behavior is verified with shared data (test vectors and golden datasets), not by duplicating logic. Risks are covered by tests of the requirements that mitigate them.

## Levels

- Unit: encoding and decoding, escaping, delta timestamps, scope checks, key hashing. Runs in each language's own test runner.
- Conformance: the vectors in `spec/protocol` run by the TypeScript, Rust, Swift and Kotlin implementations.
- Golden: the dataset in `spec/golden` is loaded into the API and dashboard clusters and every shared metric must match expected values.
- Integration: collector, queue, consumers and ClickHouse in Docker, including redelivery and queue outage cases, for each supported queue.
- Contract: each service is checked against its OpenAPI document.
- Security: key revocation propagation (within 60 seconds), closing of streams for revoked keys, rate limiting of failed attempts, scope and site enforcement.
- Privacy: inspection of events, queues and logs for IP addresses and raw identify ids; deletion across all three clusters.
- Performance: throughput and latency targets (for example 20,000 events per second at the collector) are goals that must be measured on defined hardware before they are claimed.

## Environments

Local Docker compose (minimal and full profiles) for integration, golden and privacy tests. CI runs unit, conformance and contract tests on every change and integration tests on changes to services, schema or the protocol. Performance tests run on a dedicated machine on demand.

## Exit criteria

All scenarios of the covered requirements have a passing test (`oxispec coverage` shows no unverified scenarios), `oxispec check` is clean, golden and conformance suites pass in every language, and no open risk of high impact lacks a verifying test. Known gaps are limited to the areas listed as out of scope above.
