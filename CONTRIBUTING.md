# Contributing

- Decisions are recorded as ADRs and behavior as requirements in `docs/` (see `docs/README.md`). Change them in the same pull request as the code.
- Run `pnpm test`, `cargo test --workspace` and `oxispec check` before opening a pull request.
- Cross-language behavior (wire format, metrics) is verified with the shared files in `contracts/`, never by duplicating logic.
- Commit with a no-reply email address if you want to keep your address private.

## Closing requirements in one pull request

A pull request that implements behavior closes the requirements it touches in the same change:

- **Specs first.** The ADRs and requirements are already merged (or are part of the pull request when the change is small).
- **`spec:` in the code.** Each module, handler or function that implements a requirement carries `// spec: REQ-<DOMAIN>-<ID>` (the comment leader of its language).
- **`verifies:` per scenario.** Each test names the scenarios it proves, one comment line each: `// verifies: REQ-COLLECT-GZF5.S2 (what the test shows)`. The form without a scenario number marks every scenario of the requirement as verified, so it is not used.
- **Check the report.** `oxispec trace REQ-<DOMAIN>-<ID>` shows implemented-by and verified-by for each scenario; `oxispec test-plan` lists scenarios that no test verifies.
- **Say what is left.** The pull request description lists the scenarios that stay unverified and why (for example, they belong to a service that does not exist yet). Do not add a `verifies:` line for a scenario that no test actually checks.
