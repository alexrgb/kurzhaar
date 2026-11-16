---
id: RISK-BZY4
type: risk
status: open
likelihood: medium
impact: low
owner: alexrgb
date: 2026-11-16
review:
links:
  arises-from:
    - REQ-COLLECT-8J26
  mitigated-by:
    - REQ-COLLECT-8J26
---
# RISK-BZY4: Collector clock skew misplaces events in time

## The risk

Events are dated wrongly: a batch delayed on the client lands in a later period, or a collector with a wrong clock shifts every event.

## What would trigger it

A phone offline for hours, a tab frozen in the background, or an operator host without time synchronisation.

## What it would cost

Slightly wrong hourly charts and, across midnight, events on the wrong day; a wrong collector clock shifts all of them.

## What is done about it

Event time comes from the server clock and the in-batch offsets only, never the client clock. Batches are small and sent on page hide, which bounds the delay. The documentation tells operators to run time synchronisation.
