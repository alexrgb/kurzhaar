---
id: RISK-KA8M
type: risk
status: open
likelihood: medium
impact: high
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from:
    - REQ-METRICS-9N0K
  mitigated-by:
    - REQ-METRICS-9N0K
---
# RISK-KA8M: Metrics differ between API and dashboard

## The risk

The public API and the dashboard show different values for the same metric.

## What would trigger it

A template change is applied to one cluster only, or a rollup is rebuilt incompletely.

## What it would cost

Loss of trust in the product, support load, wrong business decisions.

## What is done about it

Both clusters are generated from the same SQL templates and a golden dataset check runs in CI.
