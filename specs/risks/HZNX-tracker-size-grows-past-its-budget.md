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
---
# RISK-HZNX: Tracker size grows past its budget

## The risk

Engagement, automatic events and Web Vitals make the tracker script noticeably larger than the current 1.5 KB gzipped.

## What would trigger it

Each new collector is added without a size check.

## What it would cost

Slower pages and less appeal against Plausible and Umami scripts.

## What is done about it

The gzipped size is measured in CI and published with each release; a budget is to be set before implementation. Each collector can be switched off.
