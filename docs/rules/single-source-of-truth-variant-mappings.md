---
title: "Single Source of Truth for Variant Mappings"
doc_type: rule
brief: "Runtime-discriminated field/index mappings must be owned by one authoritative function"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Single Source of Truth for Variant Mappings

## Rule

When the meaning of a field, array slot, or UI label depends on a runtime variant like `Backend`, all reads and writes for that mapping must go through one authoritative function or method.

## Required Practice

- Keep backend-specific label order and backend-specific data extraction in the same module.
- Do not duplicate index conventions in `main`, `ui`, or call sites.
- Prefer semantic conversion methods like `FormState::to_new_profile()` over inline `fields[n]` reads.

## Why

This project already had a silent data-loss bug caused by Claude and Codex form layouts sharing the same buffer while different files assumed different meanings for the same index.

See:
- `docs/lessons/form-field-index-single-source-of-truth.md`
- `docs/procs/tdd-config-add-env-20260303164823/execution-log.md`

## Review Check

If adding a backend or changing a form layout requires updating more than one field-index mapping site, the design is wrong.
