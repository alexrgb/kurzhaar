---
id: RISK-K28G
type: risk
status: open
likelihood: low
impact: high
owner: alexrgb
date: 2026-11-15
review:
links:
  arises-from: REQ-COLLECT-BCES
---
# RISK-K28G: Raw client signals narrow a visitor down before they are discarded

## The risk

Brands, platform version, touch points, screen size and language together can single out a visitor. If they were stored or logged, they would work as a fingerprint.

## What would trigger it

A service that logs request headers or batch headers, a queue or log that keeps the raw values, or a detector plug-in that sends signals to a third party.

## What it would cost

Loss of the cookieless, privacy-first promise and possible legal exposure for the operator.

## What is done about it

Raw signals are discarded right after detection and never queued (REQ-COLLECT-RA57). The device model and build numbers are not collected at all (REQ-COLLECT-BCES). A detection service that is called over HTTP must be operated by the same deployment; this is stated in the deployment guide.
