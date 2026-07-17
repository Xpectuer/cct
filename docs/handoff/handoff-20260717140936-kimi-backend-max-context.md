# Handoff: Kimi Backend + max_context_size for cct

**Timestamp**: 2026-07-17 14:09 UTC+8
**Branch**: `master`
**HEAD**: `50780e5`
**Dirty workspace**: `true` (unstaged changes in `src/config.rs`)
**Dirty stage**: `false`

## Goal

Add a **Kimi backend** to the cct TUI launcher so users can launch the [Kimi Code CLI](https://code.kimi.com) (`kimi`) from the TUI, with **max_context_size** as a profile-level toggleable field (1m / 260k), plus **capabilities**, **display_name**, and **effort fields** in the generated `~/.kimi-code/config.toml`.

## Background

`cct` is a terminal UI launcher for Claude Code and OpenAI Codex (and now Kimi). It reads named profiles from a TOML config file, displays them in a ratatui TUI organized into backend tabs, and exec-replaces the process with the target CLI.

## What Has Been Done (Original Plan from `~/.claude/plans/breezy-drifting-ullman.md`)

The implementation was **completed once** (6 phases, 126 tests passed, clippy clean), then **wiped by a failed sed command** and partially re-applied. Current state:

### config.rs — Partially applied (dirty file)
- `Backend::Kimi` variant ✅
- `default_max_context_size()` / `resolve_max_context_size()` / `toggle_kimi_max_context_size()` helper fns ✅
- `max_context_size` field on `Profile` ✅
- `max_context_size` field on `NewProfile` ✅
- **MISSING**: `DEFAULT_CONFIG` default-kimi profile
- **MISSING**: `ensure_kimi_profile()` fn
- **MISSING**: `validate_profiles()` Kimi rules
- **MISSING**: `update_profile()` Kimi arm
- **MISSING**: `append_profile()` Kimi arms (backend name match + env section)

### app.rs — NOT touched in this attempt
- **MISSING**: `field_labels` Kimi arm (6 fields: Name/Desc/BaseURL/APIKey/Model/Context)
- **MISSING**: `from_profile` Kimi arm
- **MISSING**: `to_new_profile` Kimi arm

### launch.rs — NOT touched in this attempt
- **MISSING**: `check_kimi_installed()`, `prompt_install_kimi()`
- **MISSING**: `generate_kimi_config()` — core logic for `~/.kimi-code/config.toml`
- **MISSING**: `build_kimi_args()`, `exec_kimi()`
- **MISSING**: `build_launch_command()` Kimi dispatch

### ui.rs — NOT touched in this attempt
- **MISSING**: 3-tab bar (Claude/Codex/Kimi)
- **MISSING**: Footer for Kimi (show Space toggle)
- **MISSING**: `build_detail()` Kimi arm (show max_context_size)

### main.rs — NOT touched in this attempt
- **MISSING**: `ensure_kimi_profile()` at startup
- **MISSING**: Key `3`, 3-way Tab rotation
- **MISSING**: Enter dispatch to `exec_kimi()`
- **MISSING**: `s`/`t` no-op for Kimi
- **MISSING**: Space key to toggle `max_context_size`

### cli.rs — NOT touched in this attempt
- **MISSING**: `--backend` flag for `cct add`
- **MISSING**: `resolve_backend()`
- **MISSING**: `[kimi]` tags

## Design Decisions (from grilling session)

| Item | Decision |
|------|----------|
| `max_context_size` field | Profile-level, TUI form 6th field, values `"1m"` / `"260k"` |
| Default detection | model starts with `k3` → `"1m"` (1,000,000), else `"260k"` (262,144) |
| TUI toggle | Space key on selected Kimi profile in normal mode |
| `capabilities` | Hardcoded `["thinking", "always_thinking", "image_in", "video_in", "tool_use"]` for all |
| `display_name` | Uppercase of model name (e.g. `kimi-k2` → `"KIMI-K2"`) |
| `support_efforts` / `default_effort` | Only for `k3*` models → `["max"]` / `"max"` |
| Kimi config injection | Surgical `toml_edit` on `~/.kimi-code/config.toml`, preserve existing providers |
| Env vars in profiles.toml | `ANTHROPIC_BASE_URL`, `ANTHROPIC_API_KEY`, `ANTHROPIC_MODEL` (no Claude-specific vars) |

## What To Do Next

1. **Reset src/config.rs from git** and re-apply the full Kimi patch cleanly
2. Apply all remaining match arms + Kimi functions across all 6 source files
3. Add `max_context_size: None` to every `Profile {}` and `NewProfile {}` test constructor
4. Build → test → clippy → verify

## Verification

```bash
cargo build          # Must succeed
cargo test           # All tests pass
cargo clippy         # No warnings
cargo run -- env     # Shows [kimi] tags
```

## Failures & Lessons

1. **sed corrupted source files**: Using `sed -i '' 's/auth_type:/auth_type: None,\n max_context_size: None/'` replaced function parameter `auth_type: Option<String>` in signatures, not just struct constructors. **Lesson**: Never use broad sed on Rust source — use the Edit tool with exact string matching, or targeted perl with structural context (e.g. matching closing `}` after `auth_type`).

2. **Lost all progress twice**: The Kimi backend was fully implemented (126 tests passing) then lost. **Lesson**: Commit incremental progress frequently — don't batch all changes before committing.

## Reference: Original Plan

Full plan at `~/.claude/plans/breezy-drifting-ullman.md`.

## Reference: kimi config format

See `~/.kimi-code/config.toml` for the target format. Key sections:
```toml
[providers."<name>"]
type = "kimi"
base_url = "..."
api_key = "..."

[models."<provider>/<model>"]
provider = "<provider>"
model = "<model>"
max_context_size = 1000000
capabilities = ["thinking", "always_thinking", "image_in", "video_in", "tool_use"]
display_name = "KIMI-K2"
# Only for k3* models:
support_efforts = ["max"]
default_effort = "max"
```
