---
title: "Hotkey UI Changes Must Be Discoverable"
doc_type: rule
brief: "Any new interactive key path must update the visible UI hints and ship with coverage for discoverability"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Hotkey UI Changes Must Be Discoverable

## Rule

Any newly added key binding in the TUI must also be reflected in the footer or equivalent visible hint, and that discoverability path should be covered by a test.

## Required Practice

- Update the footer when adding a hotkey.
- Add at least one test covering either the footer hint or the dispatch path.
- Treat undocumented hotkeys as incomplete features.

## Why

This project repeatedly paired key-path changes with footer updates for a reason: terminal UIs have weak discoverability unless the interface advertises commands explicitly.

See:
- `docs/procs/tdd-continue-key-20260312172244/execution-log.md`
- `docs/procs/tdd-autoinstall-skip-perms-20260310143442/execution-log.md`
- `docs/modules/ui.md`

## Review Check

If a user cannot learn about a new key binding from the running UI, the feature is not finished.
