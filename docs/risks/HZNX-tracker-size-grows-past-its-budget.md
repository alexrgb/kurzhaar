---
id: RISK-HZNX
type: risk
status: open
likelihood: medium
impact: medium
owner: alexrgb
date: 2026-11-15
review:
links:
  arises-from: REQ-TRACKER-N7SJ
  mitigated-by: REQ-TRACKER-YDD6
---
# RISK-HZNX: Tracker size grows past its budget

## The risk

Engagement, automatic events and Web Vitals make the tracker script noticeably larger than the current 1.5 KB gzipped.

## What would trigger it

Each new collector is added without a size check.

## What it would cost

Slower pages and less appeal against Plausible and Umami scripts.

## What is done about it

The budget is 4096 bytes gzipped for the tracker with all collectors on (ADR-9G74), enforced by a test that fails the build (REQ-TRACKER-YDD6). The size is also published with each release. Each collector can be switched off.
