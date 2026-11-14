---
id: RISK-6DCB
type: risk
status: open
likelihood: medium
impact: high
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from:
    - REQ-AUTH-B798
  mitigated-by:
    - REQ-AUTH-TNJR
---
# RISK-6DCB: Revoked key still accepted through cache

## The risk

A revoked API key keeps working because a service still holds it in its in-memory cache.

## What would trigger it

A notification is lost or a service starts with a stale cache.

## What it would cost

Unauthorized reads of analytics data or a live event stream after a key leak.

## What is done about it

Revocation is pushed to all services and cache entries expire after at most 60 seconds as a safety net; a test checks both.
