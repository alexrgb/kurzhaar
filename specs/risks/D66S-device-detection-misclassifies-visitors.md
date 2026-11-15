---
id: RISK-D66S
type: risk
status: open
likelihood: medium
impact: medium
owner: alexrgb
date: 2026-11-15
review:
links:
  arises-from: REQ-COLLECT-X6WF
---
# RISK-D66S: Device detection misclassifies visitors

## The risk

A browser, operating system or device class is wrong or unknown, so reports are skewed (for example tablets counted as desktop, new browsers counted as other).

## What would trigger it

New devices and browsers appearing after the rule set was built, browsers that send no client hints, spoofed or "desktop site" modes, and bots that look like browsers.

## What it would cost

Wrong audience splits and wrong decisions based on them. Old events cannot be re-detected because raw signals are discarded.

## What is done about it

The detector is replaceable and its rules can be updated without touching trackers (ADR-ZKD1). Client hints and touch points correct the cases a User-Agent cannot. A test set of real User-Agents and signals runs in CI against the default detector.
