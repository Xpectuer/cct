---
title: "Test Boundaries With Stubs Before Manual Verification"
doc_type: rule
brief: "External integrations should be made deterministic under test before relying on manual verification"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Test Boundaries With Stubs Before Manual Verification

## Rule

For shell, network, install, and platform-facing behavior, first create deterministic automated coverage with stubs or fakes. Use manual verification only for the remaining irreducible gap.

## Required Practice

- Make scripts sourceable when they need function-level tests.
- Stub external commands before reaching for live calls.
- Convert "manual-only" checks into mocked tests whenever the behavior is actually mockable.

## Why

The install script work succeeded because live-ish shell behavior was reduced to BATS stubs instead of being left as manual QA. Manual verification still had a place, but not as the first line of defense.

See:
- `docs/lessons/bats-shell-function-stubbing.md`
- `docs/procs/tdd-install-script-20260310150440/execution-log.md`
- `docs/procs/tdd-autoinstall-skip-perms-20260310143442/tdd.md`

## Review Check

If a behavior depends on `curl`, `uname`, `sleep`, `tar`, or CLI presence and has no stub-driven test path, push toward testability first.
