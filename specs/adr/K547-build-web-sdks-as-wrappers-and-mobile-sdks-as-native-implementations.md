---
id: ADR-K547
type: adr
status: accepted
---
# ADR-K547: Build web SDKs as wrappers and mobile SDKs as native implementations

## Context

Targets are the browser, Node, React, React Native, Flutter, iOS and Android. Browsers and Node share JavaScript. Mobile needs offline queues, background flushing and a stable install identifier, which are platform-specific. Writing five independent implementations multiplies the chance of protocol drift.

## Decision

- The browser tracker (`packages/tracker`) is the reference web implementation. The React SDK is a thin wrapper over it. The Node SDK is a separate small server-side implementation of the same protocol.
- Swift (`sdks/ios`) and Kotlin (`sdks/android`) are real native implementations with a disk-backed offline queue and background flush.
- React Native and Flutter wrap the native SDKs instead of reimplementing them.
- Every implementation passes the shared protocol vectors in `spec/protocol`.
- Swift Package Manager needs `Package.swift` at the repository root, which points to `sdks/ios`, and plain semantic version tags. Other components use prefixed tags (for example `tracker@1.0.0`). Whether SPM tolerates the prefixed tags must be verified at the first release; a mirror repository is the fallback.

## Consequences

- Fewer independent protocol implementations (TypeScript, Rust, Swift, Kotlin).
- React Native and Flutter releases depend on the native SDK versions.
- Mobile identifiers must follow the privacy decisions (no cross-app tracking, no advertising IDs).
