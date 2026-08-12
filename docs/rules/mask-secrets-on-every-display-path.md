---
title: "Mask Secrets on Every Display Path"
doc_type: rule
brief: "Any user-visible rendering of secret-like config values must apply the shared masking policy"
confidence: verified
created: 2026-03-22
updated: 2026-08-12
revision: 2
---

# Mask Secrets on Every Display Path

## Rule

Any UI, confirmation summary, detail panel, or debug-facing display of config/env values must mask secret-like keys using the shared masking policy.

## Required Practice

- Route each display surface through its designated masking helper instead of printing raw secrets or open-coding redaction (`ui::mask_value` in the TUI and `cli::mask_key` in CLI summaries).
- Apply masking in confirmation views as well as steady-state detail views.
- When the definition of sensitive data changes, update every surface-specific helper and its tests in the same changeset.

## Why

The TUI treats env keys containing `TOKEN`, `KEY`, or `SECRET` as sensitive, while the CLI add summary masks the API-key value explicitly. Bypassing those helpers creates inconsistent leakage risk.

See:
- `docs/modules/ui.md`
- `docs/procs/tdd-config-add-env-20260303164823/execution-log.md`
- `docs/procs/tdd-cct-tui-launcher-20260302142400/tdd.md`

## Review Check

If a new screen or summary prints env or credential-like fields without going through the masking helper for that surface, it violates this rule.
