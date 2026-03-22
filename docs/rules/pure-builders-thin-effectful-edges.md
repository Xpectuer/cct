---
title: "Pure Builders, Thin Effectful Edges"
doc_type: rule
brief: "Keep data shaping pure and isolate I/O, env mutation, terminal changes, and exec in narrow wrappers"
confidence: verified
created: 2026-03-22
updated: 2026-03-22
revision: 1
---

# Pure Builders, Thin Effectful Edges

## Rule

Separate pure decision logic from side effects. Builders should return values; wrappers should perform I/O, env mutation, terminal cleanup, or `exec`.

## Required Practice

- Keep argument construction in pure helpers.
- Keep config serialization/generation testable without process replacement.
- Isolate terminal restoration and process launch in dedicated functions.

## Why

This pattern is already one of the project's strengths. It is what makes launch behavior, config generation, and edge-case coverage easy to test.

See:
- `docs/modules/launch.md`
- `docs/procs/tdd-continue-key-20260312172244/execution-log.md`
- `docs/procs/tdd-cct-tui-launcher-20260302142400/tdd.md`

## Review Check

If a function both computes argument/config state and performs irreversible side effects, split it unless the combined shape is trivially small.
