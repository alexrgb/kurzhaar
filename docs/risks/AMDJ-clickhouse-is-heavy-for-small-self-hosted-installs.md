---
id: RISK-AMDJ
type: risk
status: open
likelihood: medium
impact: medium
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from: ADR-M1DT
---
# RISK-AMDJ: ClickHouse is heavy for small self-hosted installs

## The risk

Small sites must run ClickHouse, which needs more memory and operations effort than a simple database.

## What would trigger it

A user tries the product on a small server.

## What it would cost

Fewer adopters, support questions.

## What is done about it

A minimal compose profile with conservative settings; a lightweight storage mode may be reconsidered later.
