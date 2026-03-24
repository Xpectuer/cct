## Step 1 — Update README default path guidance

### Actions Taken
- Documented the macOS default config path and the Linux/other Unix path in the Quick Start droplet.
- Updated the manual edit option to call out the same platform-specific locations.

### Verify Result
- `rg -n 'Library/Application Support/cc-tui/profiles.toml|~/.config/cc-tui/profiles.toml' README.md` (exit 0)
