---
title: "Spec: macOS default config path documentation fix"
doc_type: proc
brief: "Clarify macOS and non-macOS default config paths in README and CLAUDE.md"
confidence: verified
created: 2026-03-24
updated: 2026-03-24
revision: 1
---

# Spec: macOS default config path documentation fix

## Solution Summary

Update the repository documentation so it no longer presents `~/.config/cc-tui/profiles.toml` as the default path on macOS. The change stays documentation-only and aligns [README.md](/home/zhengjy/workspace/cc_starter/README.md) with [CLAUDE.md](/home/zhengjy/workspace/cc_starter/CLAUDE.md) by using the same platform-specific guidance: macOS uses `~/Library/Application Support/cc-tui/profiles.toml`, while non-macOS Unix-like platforms can continue to reference `~/.config/cc-tui/profiles.toml` where applicable.

## Acceptance Criteria

- [ ] README explicitly states that the macOS default config path is `~/Library/Application Support/cc-tui/profiles.toml`.
- [ ] CLAUDE.md explicitly states that the macOS default config path is `~/Library/Application Support/cc-tui/profiles.toml`.
- [ ] README and CLAUDE.md use consistent platform-specific wording and no longer imply that `~/.config/cc-tui/profiles.toml` is the default on macOS.
- [ ] The change remains documentation-only for this first version.
