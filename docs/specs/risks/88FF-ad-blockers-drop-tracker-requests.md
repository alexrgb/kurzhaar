---
id: RISK-88FF
type: risk
status: open
likelihood: high
impact: medium
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from: REQ-COLLECT-C50Y
---
# RISK-88FF: Ad blockers drop tracker requests

## The risk

Browsers with content blockers prevent the tracker script or its requests, so counts are too low.

## What would trigger it

Default script and endpoint names match public blocker lists.

## What it would cost

Under-counted traffic, lower value of the product.

## What is done about it

Planned: first-party endpoints, configurable script and endpoint names, a framework rewrite helper and server-side tracking. Not yet specified as requirements.
