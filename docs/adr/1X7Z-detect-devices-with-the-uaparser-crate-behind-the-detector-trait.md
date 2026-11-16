---
id: ADR-1X7Z
type: adr
status: accepted
links:
  refines:
    - ADR-ZKD1
---
---
# ADR-1X7Z: Detect devices with the uaparser crate behind the Detector trait

## Context

ADR-ZKD1 decides that the collector detects browser, OS, device class and bots behind a replaceable interface, with a default built on the uap-core rule set. This record picks the default implementation and fixes the interface.

## Decision

- **Interface.** A `Detector` trait in `crates/core` takes the signals (User-Agent, Client Hints headers, and the tracker's brands, mobile flag, platform, major platform version, touch points and pointer type) and returns browser name and major version, OS name and major version, device class (`desktop`, `mobile`, `tablet`, `other`) and a bot flag. It performs no I/O and holds no state except its rules.
- **Default implementation.** The `uaparser` crate (MIT; it uses the uap-core regular expressions, Apache-2.0) parses the User-Agent. Our own code applies the class logic over the signals: client-hint platform and mobile flag win over the User-Agent; Windows with a platform major version of 13 or more is Windows 11; a Mac User-Agent with more than one touch point is an iPad; touch points and a coarse pointer separate a tablet from a desktop when hints are missing; versions are cut to the major part.
- **Bots.** The parser's spider device family plus a short list of our own patterns (crawler, headless and monitoring tools) set the bot flag. Any external bot list is adopted only after its licence is checked and recorded here.
- **Cache.** A bounded in-memory map from the tuple of signals to the result (REQ-COLLECT-4THR). The size and expiry are configuration with defaults of 10,000 entries and one hour.
- **Other detectors.** The configuration selects the detector by name. `builtin` is the only one in version 1; an HTTP detector and a database-backed one are possible later without a change to the tracker or the envelope.
- **Rules are data.** Rule updates arrive with the crate; the repository keeps a table of real User-Agents and signals with the expected result, run in CI (the device detection level of the test plan).

## Consequences

- No LGPL or commercial code is linked into the MIT core.
- Accuracy follows the rule set's freshness: a collector upgrade refreshes the rules, and old events keep their classification (ADR-ZKD1).
- The class logic is ours to maintain; the fixture table is what keeps it honest (RISK-D66S).
- `uaparser` loads its regular expressions at start, which costs some memory and start time; this is measured before release.
- The crate's maintenance status is checked again before the first release, and the trait is the escape hatch if it stalls.
