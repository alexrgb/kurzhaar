---
id: RISK-SQGK
type: risk
status: open
likelihood: low
impact: medium
owner: alexrgb
date: 2026-11-16
review:
links:
  arises-from: REQ-TRACKER-KE2V
---
# RISK-SQGK: Timezone offset adds to the visitor's signal set

## The risk

The timezone offset is one more value that, together with screen size, language and device class, narrows a visitor down.

## What would trigger it

Storing the offset next to other signals in a way that allows a join, or adding the IANA zone name later, which is far more specific than an offset.

## What it would cost

A weaker cookieless, privacy-first promise and possible legal exposure for the operator.

## What is done about it

Only the offset in minutes is collected, not the zone name (ADR-6RMH). It has a small set of values shared by very many people. No identifier is built from it and the raw device signals are still discarded (REQ-COLLECT-RA57).
