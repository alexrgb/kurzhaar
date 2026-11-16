---
id: DEBT-DWPW
type: debt
status: open
severity: medium
owner: alexrgb
date: 2026-11-14
due:
cost:
interest:
links:
  arises-from: ADR-XET8
---
# DEBT-DWPW: Export of raw events older than 30 days is not designed

## What it is

The API and dashboard clusters keep raw events for 30 days only. Exporting older events has to read the RAW cluster, but no design exists yet for how a client requests it (an asynchronous job, where the result is stored, size limits, formats).

## Why it was taken

The first version focuses on ingestion, rollups and 30-day export. Long-range export is a separate feature with its own job and storage design, and was deferred on purpose (decision of 2026-11-14, to be revisited when documentation and scaffolding resume).

## What paying it off takes

Design an asynchronous export job (request, status, download link), choose where result files live, define formats (NDJSON, CSV, Parquet) and limits, and add scopes and rate limits. Risk: heavy reads on the RAW cluster, so the job needs throttling.
