---
id: RISK-FP70
type: risk
status: open
likelihood: low
impact: high
owner: alexrgb
date: 2026-11-16
review:
links:
  arises-from:
    - REQ-PRIVACY-HNMK
  mitigated-by:
    - REQ-PRIVACY-E4TN
    - REQ-COLLECT-8ERQ
---
# RISK-FP70: Salt storage lets an operator link visitors across days

## The risk

An operator, a backup or a bug keeps old salts, so session hashes of earlier days can be recomputed from addresses and User-Agents and visitors can be followed across days.

## What would trigger it

Salts kept in database backups, salts derived from a fixed secret, or logs that hold addresses next to hashes.

## What it would cost

The privacy promise of the product no longer holds, and visits spanning days could be reconstructed.

## What is done about it

Salts are random, one per UTC day, deleted ten minutes after their day, and never derived from a long-lived secret. The privacy tests check tables, caches and backups' source tables for old salts. Backups of the salt table are the operator's responsibility and the documentation says so.
