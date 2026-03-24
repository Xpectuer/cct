---
title: "Plan: macOS default config path documentation fix"
doc_type: proc
brief: "Update README and CLAUDE.md to document platform-specific default config paths"
confidence: verified
created: 2026-03-24
updated: 2026-03-24
revision: 1
---

# Plan: macOS default config path documentation fix

## Files Changed

| File | Change Type |
|------|-------------|
| `README.md` | Minor edit |
| `CLAUDE.md` | Minor edit |

## Step 1 — Update README default path guidance

**File**: `README.md`
**What**: Rewrite the Quick Start path guidance so README explicitly distinguishes the macOS default config path from the non-macOS path and keeps the manual-edit instructions consistent with that wording.

**Old**:
```md
## Quick Start
```

**New**:
```md
Revise the `Quick Start` section so the generated config path is documented by platform, with macOS using `~/Library/Application Support/cc-tui/profiles.toml` and Linux or other Unix-like platforms using `~/.config/cc-tui/profiles.toml`. Update the manual-edit option in the same section to reference the same platform-specific paths instead of a single universal path.
```

**Verify**: `rg -n 'Library/Application Support/cc-tui/profiles.toml|~/.config/cc-tui/profiles.toml' README.md`

## Step 2 — Update CLAUDE.md platform-specific config path references

**File**: `CLAUDE.md`
**What**: Replace generic config path wording in the overview and config format sections so CLAUDE.md matches README and explicitly documents the macOS path.

**Old**:
```md
## Project Overview
```

**New**:
```md
Revise `Project Overview` so it no longer says the config file is always `~/.config/cc-tui/profiles.toml`, and revise `Config File Format` so the location text is platform-specific. The final wording must state that macOS uses `~/Library/Application Support/cc-tui/profiles.toml` and must not imply that `~/.config/cc-tui/profiles.toml` is the macOS default.
```

**Verify**: `rg -n 'Library/Application Support/cc-tui/profiles.toml|~/.config/cc-tui/profiles.toml' CLAUDE.md`

## Step 3 — Proof-Read End-to-End

Read each changed file in full. Check: formatting, no leftover TODOs, spec intent preserved.

## Step 4 — Cross-Check Acceptance Criteria

| Criterion | Addressed in Step |
|-----------|------------------|
| README explicitly states that the macOS default config path is `~/Library/Application Support/cc-tui/profiles.toml`. | Step 1 |
| CLAUDE.md explicitly states that the macOS default config path is `~/Library/Application Support/cc-tui/profiles.toml`. | Step 2 |
| README and CLAUDE.md use consistent platform-specific wording and no longer imply that `~/.config/cc-tui/profiles.toml` is the default on macOS. | Step 1, Step 2, Step 3 |
| The change remains documentation-only for this first version. | Step 1, Step 2, Step 3 |

## Step 5 — Review

Follow Phase 3 (see `03-self-review.md`). Writes `review.md`.

## Step 6 — Commit

Use `/commit`. Suggested message:

`docs: clarify macOS config path defaults`

- update README quick-start and manual-edit path guidance
- align CLAUDE.md config path wording with macOS behavior

## Execution Order

Step 1 → Step 2 → Step 3 → Step 4 → Step 5 → Step 6
