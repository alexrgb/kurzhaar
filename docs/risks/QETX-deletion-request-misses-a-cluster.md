---
id: RISK-QETX
type: risk
status: open
likelihood: low
impact: high
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from:
    - REQ-PRIVACY-0RVD
  mitigated-by:
    - REQ-PRIVACY-0RVD
---
# RISK-QETX: Deletion request misses a cluster

## The risk

A deletion request removes personal data from some clusters but not all three.

## What would trigger it

One cluster is unreachable or the job fails midway.

## What it would cost

Legal exposure and loss of user trust.

## What is done about it

Deletion is an audited job that tracks per-cluster completion and is retried until all clusters confirm.
