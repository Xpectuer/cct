---
title: "Mask Secrets on Every Display Path"
doc_type: rule
brief: "Any user-visible rendering of secret-like config values must apply the shared masking policy"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Mask Secrets on Every Display Path

## Rule

Any UI, confirmation summary, detail panel, or debug-facing display of config/env values must mask secret-like keys using the shared masking policy.

## Required Practice

- Reuse one masking helper instead of open-coding redaction.
- Apply masking in confirmation views as well as steady-state detail views.
- Expand the shared policy centrally if the definition of sensitive keys changes.

## Why

This project already treats `TOKEN`, `KEY`, and `SECRET` as sensitive display paths. Re-implementing that logic locally creates inconsistent leakage risk.

See:
- `docs/modules/ui.md`
- `docs/procs/tdd-config-add-env-20260303164823/execution-log.md`
- `docs/procs/tdd-cct-tui-launcher-20260302142400/tdd.md`

## Review Check

If a new screen prints env or credential-like fields without going through `mask_value`, it violates this rule.
