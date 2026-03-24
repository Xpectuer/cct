## Step 4 — Cross-Check Acceptance Criteria

### Actions Taken
- Re-ran `rg` on README.md and CLAUDE.md to confirm both document `~/Library/Application Support/cc-tui/profiles.toml` for macOS alongside `~/.config/cc-tui/profiles.toml` for Linux/Unix.
- Checked the tracked changes to confirm only documentation files were touched as part of this fix.

### Verify Result
- `rg -n 'Library/Application Support/cc-tui/profiles.toml|~/.config/cc-tui/profiles.toml' README.md` (exit 0)
- `rg -n 'Library/Application Support/cc-tui/profiles.toml|~/.config/cc-tui/profiles.toml' CLAUDE.md` (exit 0)
- `git status --short` (exit 0) – shows only doc files were modified.
