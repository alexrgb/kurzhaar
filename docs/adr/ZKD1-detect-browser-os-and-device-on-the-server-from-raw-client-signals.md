---
id: ADR-ZKD1
type: adr
status: accepted
links:
  refines:
    - ADR-B0B4
---
# ADR-ZKD1: Detect browser, OS and device on the server from raw client signals

## Context

ADR-B0B4 adds browser, operating system and device class to every event. A tracker script cannot classify a device reliably: it is tiny, it sees only what the browser exposes, and detection rules (new browsers, new devices, bots) change every month, which would mean redeploying every customer's script. The User-Agent alone is not enough either: Windows 10 and 11 both say `Windows NT 10.0`, and iPad Safari says `Macintosh`. The better signals come from the browser: User-Agent Client Hints (`navigator.userAgentData`, Chromium only), touch points and pointer type.

## Decision

Collection and detection are separate:

- **The tracker only collects raw signals** and sends them once per batch in the header line, as optional wire fields (exact keys are assigned in the field registry, ADR-VJGJ): client-hint brands with major versions, mobile flag, platform, platform version, maximum touch points and whether the primary pointer is coarse. Screen size is already sent. The device model and other high-entropy values (architecture, bitness, full build versions) are not collected. The tracker does no classification.
- **The collector detects** from the User-Agent header, the Client Hints headers the browser sends by itself, and the tracker's signals. Client-hint signals take priority over the User-Agent where both exist.
- **The detector is replaceable.** The collector calls a `Detector` interface that takes the signals and returns browser, browser major version, operating system, operating system major version and device class (`desktop`, `mobile`, `tablet`, `other`), plus a bot flag. The default implementation is built on the uap-core rule set (Apache-2.0) with our own class logic on top of the signals. Other implementations may be plugged in by configuration: a separate detection service called over HTTP, or a commercial or LGPL database that the operator installs (a separate component, so the MIT core carries no LGPL code).
- **Results are cached** by the tuple of signals, because User-Agent strings repeat heavily; the detector is never on the per-event hot path for a seen tuple.
- **Bots** are flagged by the detector and excluded from metrics by default, not counted as `other`.
- **Raw signals are never stored or queued.** The collector discards the User-Agent, Client Hints and the tracker's raw signals right after detection, as ADR-B0B4 already requires for the User-Agent. Only the detected classes and major versions are enqueued.

## Consequences

- Detection rules can improve without touching any tracker; a collector upgrade or a detector swap is enough.
- Old events keep the classification made at ingestion; they are not re-detected, because the raw signals are discarded on purpose.
- The tracker grows by a few dozen bytes and reads `navigator.userAgentData` asynchronously for the high-entropy values; the size is measured under RISK-HZNX.
- Firefox and Safari send no Client Hints, so for them the User-Agent and touch points are all there is. iPad in desktop mode is told apart by touch points and the primary pointer.
- A misclassification is possible whatever the detector (RISK-D66S), and sending more signals narrows a visitor down before they are discarded (RISK-K28G); the second is handled by never storing them.
- Whether any of this is personal data depends on the deployment; this is not legal advice.
