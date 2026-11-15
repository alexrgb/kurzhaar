---
id: RISK-RFN7
type: risk
status: open
likelihood: medium
impact: low
owner: alexrgb
date: 2026-11-15
review:
links:
  arises-from: REQ-METRICS-JEEQ
---
# RISK-RFN7: Visits are split after a consumer restart

## The risk

A consumer restart loses the in-memory visit state, so one long visit is counted as two.

## What would trigger it

A crash or deploy while visitors are active, with an incomplete state rebuild.

## What it would cost

Slightly higher visit counts and bounce rates around restarts.

## What is done about it

The consumer rebuilds state from the last 30 minutes of its cluster on start (ADR RWB4). Remaining splits are accepted; no mitigating requirement yet.
