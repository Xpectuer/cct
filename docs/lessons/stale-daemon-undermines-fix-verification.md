---
title: "Stale Daemon Undermines Fix Verification — Probe Coverage Must Equal the Fault Surface"
doc_type: lesson
brief: "After fixing a long-running daemon bug, a healthy-looking probe can keep reusing the stale instance; verify the running process is the fixed binary and that the probe covers the fixed fault surface"
confidence: verified
created: 2026-08-04
updated: 2026-08-04
revision: 1
---

# Lesson: Stale Daemon Undermines Fix Verification

## Context

The proxy deadlock fix (async accept for the control socket, PR #13) passed
195/195 tests, including a contract test that reproduces the original hang.
Yet the user reported: "启动 codex profile 仍然卡住" — still stuck after the
fix landed.

## The Bug

On the user machine the symptoms were:

- `status` control command over the unix socket → immediate healthy response
- HTTP request to the proxy → hangs 10s+ with no response
- Direct request to the upstream → 200 in 0.39s

Timeline forensics:

1. `~/.local/bin/cct` binary built **Aug 1 14:41** — 12 hours *before* the fix
   commit (Aug 2 02:42).
2. The proxy daemon (PID 88476) was started **Aug 3 20:30** with that stale
   binary.
3. The new code (`ensure_proxy_running`) probes the daemon with an
   application-level `status` command, finds it "healthy", and **reuses it** —
   so codex requests kept flowing into the old daemon whose HTTP layer was
   still starved by the sync-accept deadlock.

The deadlock only starves the HTTP accept loop; the control channel stays
responsive (control connections wake the blocking accept). So the app-level
probe — which only covers the control channel — correctly reports health while
the actual broken surface (HTTP forwarding) remains dead.

## Root Cause

Two compounding traps:

1. **A long-running daemon is not replaced by shipping a new binary.** The fix
   lives in the code, but the running process was spawned from the old binary
   and keeps running until stopped.
2. **Probe coverage ≠ fault surface.** `check_proxy_running` proves the control
   channel answers; it says nothing about the HTTP layer. "Reuse if healthy"
   logic assumed the probe covers the whole proxy, which it does not.

## The Fix

- Stop the stale daemon (`cct proxy stop` / kill); the next launch spawns the
  fixed binary and self-heals the leftover socket file.
- Verified the fixed binary in isolation (separate socket + port): control and
  HTTP forwarding both respond normally.

## Rule Derived

> When a fix targets a long-running daemon/agent/service, verifying the fix
> means confirming the *running instance* is the fixed binary — check process
> start time / binary mtime / version — not just that tests pass.
> And a "reuse if healthy" probe must cover the exact fault surface being
> fixed, or the stale instance keeps serving the bug.

## Symptoms to Watch For

- All tests green, but the user still reproduces the original bug.
- Control/management commands respond while the data path hangs.
- Process start time predates the fix commit's build time.

## Related

- `docs/procs/tdd-proxy-deadlock-fix-20260801172308/` — the TDD session that fixed the deadlock
- `src/proxy.rs` — `check_proxy_running` / `run_control_socket`
- `src/launch.rs` — `ensure_proxy_running` reuse logic
