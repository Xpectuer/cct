# CLAUDE.md Snapshot

Project: `cct` — terminal UI launcher for Claude Code and OpenAI Codex.
Five modules: config, app, ui, launch, cli.
Rust codebase with ratatui TUI, TOML config, exec-replace architecture.
Key: Codex launch uses `generate_codex_config` writing to `~/.config/cct-tui/codex/config.toml`, `CODEX_HOME` set before exec.
Config hot-reload on `e` key. `s` toggles skip_permissions via toml_edit.
