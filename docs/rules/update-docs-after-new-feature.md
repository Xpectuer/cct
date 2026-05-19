# Update Docs After New Feature

## Rule

After implementing a new feature (new hotkey, new CLI flag, new config field, new toggle, new backend behavior), update all relevant documentation in the same changeset:

1. **README.md** — user-facing features list, keybindings table, subcommands table, config example
2. **ARCHITECTURE.md** — Configuration-Driven Logic table, Critical Path flows, Key Design Decisions
3. **CLAUDE.md** — module description table, key design choices, config file format example
4. **docs/modules/*.md** — interface sections (new types, fields, functions), edge cases, usage examples

## Why

The auth_type feature was initially merged with code changes only. README keybindings, ARCHITECTURE config table, and module docs were all stale — missing the `t` hotkey, `--auth-type` flag, and `toggle_auth_type` function. This creates drift between what the code does and what the docs say.

## How to Apply

When wrapping up a feature:
- Audit `README.md` — is the new hotkey in the keybinding table? Is the new CLI flag in the subcommands table?
- Audit `ARCHITECTURE.md` — is the new config field in the Configuration-Driven Logic table? Is the new flow in Critical Path?
- Audit `CLAUDE.md` — are module descriptions up to date? Is the config example current?
- Audit `docs/modules/<touched>.md` — are new struct fields, functions, and edge cases documented?
