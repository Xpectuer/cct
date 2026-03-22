---
title: "Preserve User-Edited Config Structure"
doc_type: rule
brief: "Mutations to user config should preserve comments, formatting, and ordering whenever possible"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Preserve User-Edited Config Structure

## Rule

When modifying `profiles.toml`, prefer surgical edits that preserve existing comments, whitespace, and key order. Full rewrites are the exception, not the default.

## Required Practice

- Use targeted editing tools like `toml_edit` for in-place field updates.
- Append new profile blocks instead of normalizing and rewriting the whole file.
- Treat user comments and hand formatting as part of the file's value.

## Why

This repository already depends on preserving config readability during toggles and incremental additions. Rewriting the whole file would discard user intent and make hot-edit workflows worse.

See:
- `docs/modules/config.md`
- `docs/procs/tdd-autoinstall-skip-perms-20260310143442/execution-log.md`

## Review Check

If a change touches one config field but rewrites unrelated formatting or comments, it violates this rule unless no safer mechanism exists.
