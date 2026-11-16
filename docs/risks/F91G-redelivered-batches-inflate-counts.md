---
id: RISK-F91G
type: risk
status: open
likelihood: medium
impact: medium
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from:
    - REQ-METRICS-63R7
  mitigated-by:
    - REQ-METRICS-63R7
---
# RISK-F91G: Redelivered batches inflate counts

## The risk

The queue delivers a batch twice and events are counted twice.

## What would trigger it

Consumer crash before acknowledging, rebalance, or producer retry.

## What it would cost

Overstated metrics that cannot be corrected without rebuilding rollups.

## What is done about it

Every insert carries a deduplication token derived from the batch; redelivery tests are part of the integration suite.
