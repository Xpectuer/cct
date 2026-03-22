---
title: "Cross-Module Features Need Contract Tests"
doc_type: rule
brief: "Features that span app, ui, config, main, and launch must be verified at the shared contract boundaries"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Cross-Module Features Need Contract Tests

## Rule

When one feature changes multiple modules, tests must cover the contract between them, not just isolated local helpers.

## Required Practice

- Add regression tests where data flows across module boundaries.
- Cover the user-visible path when a bug class depends on agreement between modules.
- Use integration or targeted cross-module tests when unit tests alone would miss drift.

## Why

The field-index mismatch and launch hotkey work both show that local correctness is not enough when `app`, `ui`, `main`, and `config` must agree on one shared behavior.

See:
- `docs/lessons/form-field-index-single-source-of-truth.md`
- `docs/procs/tdd-config-add-env-20260303164823/execution-log.md`
- `docs/procs/cct-e2e-verification-20260302140000/progress.md`

## Review Check

If each touched module has a passing unit test but the end-to-end user flow could still drift, add a contract test.
