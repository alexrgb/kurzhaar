---
id: RISK-WWXV
type: risk
status: open
likelihood: low
impact: critical
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from:
    - REQ-PRIVACY-YGHB
  mitigated-by:
    - REQ-PRIVACY-YGHB
---
# RISK-WWXV: IP address leaks into logs or storage

## The risk

Client IP addresses end up in application logs, queues or database fields.

## What would trigger it

Verbose request logging, proxy access logs or a field copied by mistake.

## What it would cost

Personal data stored without need, legal exposure.

## What is done about it

The collector discards the address before enqueueing, and tests inspect events and logs for IP patterns.
