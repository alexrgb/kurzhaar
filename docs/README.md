# Documentation

Project knowledge lives as linked nodes managed by [oxispec](https://github.com/alexrgb/oxispec). Nodes are Markdown files; IDs are random (`REQ-AUTH-ZDDT`) so parallel branches do not clash.

| Where | What |
|---|---|
| `docs/specs/adr/` | Architecture decision records: why the architecture is what it is |
| `docs/specs/requirements/<domain>/` | Requirements with scenarios (domains: protocol, collect, tracker, privacy, auth, metrics) |
| `docs/specs/test-plans/` | Test plans and which requirements and risks they cover |
| `docs/specs/risks/` | Risk register |
| `docs/specs/debt/` | Known shortcuts and deferred decisions |

Useful commands (run from the repository root):

```bash
oxispec check          # structural checks
oxispec status         # summary
oxispec coverage       # requirements without implementation or tests
oxispec show REQ-AUTH-ZDDT
oxispec risks          # risk register, most serious first
```

Tests and code reference requirements with `spec:` and `verifies:` comments (see `oxispec trace`).
