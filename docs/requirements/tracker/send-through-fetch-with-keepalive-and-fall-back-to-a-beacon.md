---
id: REQ-TRACKER-7NVW
type: requirement
status: draft
links:
  decided-by:
    - ADR-RH0Y
---
### Requirement: Send through fetch with keepalive and fall back to a beacon
The system SHALL send batches with `fetch` and `keepalive` so the answer can be read, use `sendBeacon` where `fetch` is missing and for the flush when the page is being unloaded, and keep the request free of a CORS preflight.

#### Scenario: Normal send {#S1}
- **WHEN** a batch is sent while the page is visible
- **THEN** it goes through `fetch` with `keepalive: true`, `credentials: 'omit'` and a `text/plain` body

#### Scenario: Page hidden {#S2}
- **WHEN** the page becomes hidden or is unloaded with batches waiting
- **THEN** every waiting batch is sent once with `sendBeacon` and no retry is scheduled

#### Scenario: No fetch {#S3}
- **WHEN** `fetch` is not available
- **THEN** the batch is sent with `sendBeacon`

#### Scenario: No preflight {#S4}
- **WHEN** a batch is sent with or without the age parameter
- **THEN** the request is a simple cross-origin request with no custom header
