---
id: RISK-3B2M
type: risk
status: open
likelihood: medium
impact: high
owner: alexrgb
date: 2026-11-14
review:
links:
  arises-from:
    - REQ-PROTOCOL-6NJT
  mitigated-by:
    - REQ-PROTOCOL-6NJT
---
# RISK-3B2M: Protocol implementations diverge between languages

## The risk

The TypeScript, Rust, Swift and Kotlin implementations disagree on encoding details such as escaping or deltas.

## What would trigger it

A format change is implemented in one language and not tested in the others.

## What it would cost

Corrupted or dropped events from some clients, hard to diagnose.

## What is done about it

A shared set of test vectors in spec/protocol must pass in every implementation in CI.
