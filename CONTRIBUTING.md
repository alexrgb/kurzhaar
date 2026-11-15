# Contributing

- Decisions are recorded as ADRs and behavior as requirements in `specs/` (see `docs/README.md`). Change them in the same pull request as the code.
- Run `pnpm test`, `cargo test --workspace` and `oxispec check` before opening a pull request.
- Cross-language behavior (wire format, metrics) is verified with the shared files in `contracts/`, never by duplicating logic.
- Commit with a no-reply email address if you want to keep your address private.
