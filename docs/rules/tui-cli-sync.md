# TUI-CLI Sync

## Rule

When adding or modifying a TUI feature (hotkey, toggle, setting, action on a profile), always review whether the CLI surface needs a corresponding option, flag, or subcommand. If it does, add it in the same changeset — don't leave it as a follow-up.

## Required Practice

- For every new TUI hotkey that modifies profile state, ask: "Can this be done via `cct add --<flag>` or a new subcommand?"
- For every new profile field writeable from the TUI, ensure `cct add` or an equivalent CLI path can populate it.
- Keep the TUI form backend (`FormState::to_new_profile`) and CLI (`cli::run_add`) in sync on which fields are writable.

## Why

The auth_type toggle (`t` hotkey) was initially implemented without CLI support. The `--auth-type` flag was added retroactively because users who script profile creation need parity with interactive workflows.

## Review Check

If a PR adds a TUI key binding that changes profile state but the `cct add` help text doesn't mention a corresponding flag, it's incomplete.
