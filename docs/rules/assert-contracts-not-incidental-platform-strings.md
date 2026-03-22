---
title: "Assert Contracts, Not Incidental Platform Strings"
doc_type: rule
brief: "Verification should target the real platform contract rather than brittle wording or environment-specific text"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Assert Contracts, Not Incidental Platform Strings

## Rule

Tests and CI checks must assert the real contract being relied on, not a fragile exact output string that varies across platforms or runners.

## Required Practice

- Match equivalence classes of valid output when tooling differs across environments.
- Prefer checking binary loadability, static-linking class, or exit-code semantics over exact wording.
- When a runner proves an assumption wrong, broaden the assertion to the real contract.

## Why

The musl CI work had two concrete failures caused by over-specific assumptions: `file` output wording and smoke-test exit expectations. The fix was to assert the contract more accurately.

See:
- `docs/procs/musl-static-linking-20260313230209/progress.md`
- `docs/procs/musl-static-linking-20260313230209/execution-log.md`

## Review Check

If a CI assertion is likely to fail because of benign tool wording or distro variation, it is too specific.
