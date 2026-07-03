---
title: "cct env can inject env vars into happy daemon"
doc_type: lesson
brief: "Use cct env <profile> -- happy daemon start to seed daemon-spawned sessions with profile environment variables"
confidence: verified
created: 2026-07-03
updated: 2026-07-03
revision: 1
claude_md_coverage: false
tags: [happy, daemon, env, cct, trick]
---

# cct env Can Inject Env Vars Into happy Daemon

## Problem

`cct env <profile> -- happy --yolo` works — profile env vars are injected, `happy` picks them up, and Claude Code runs with the right API endpoint and model. But what about daemon mode? When you start a session from your phone via the daemon, do the profile env vars still apply?

## Answer: Yes, via Unix process inheritance

```
cct env <profile> -- happy daemon start
```

This starts the daemon as a child of `cct env`, so the daemon inherits all injected env vars. Any session the daemon spawns later inherits them in turn.

## The Chain

```
cct env aliyun-bailian -- happy daemon start
  │
  └─ exec_with_env(): set ANTHROPIC_BASE_URL, ANTHROPIC_MODEL, etc.
      └─ exec bash -c "happy daemon start"
           └─ daemon (PID=N, PPid=1 after detach)
                └─ daemon spawns Claude Code session
                     └─ claude inherits all env vars ✓
```

No special flags, no config — just standard Unix `fork`/`exec` environment inheritance.

## Verification

After starting the daemon via `cct env`, check the daemon process environment:

```bash
# Get daemon PID
cat ~/.happy/daemon.state.json | jq .pid

# Inspect its env
cat /proc/<pid>/environ | tr '\0' '\n' | grep ANTHROPIC
```

All profile env vars should be present.

## How to Use

```bash
# 1. Stop any running daemon
happy daemon stop

# 2. Start daemon with profile env vars injected
cct env <profile> -- happy daemon start

# 3. All daemon-spawned sessions (including mobile) now use the profile's API endpoint, model, auth, etc.
```

## Why This Works

- `cct env` uses `exec()` (not `spawn`), so the target process inherits the modified environment directly.
- `happy daemon start` forks to background but does not sanitize the environment.
- Child processes inherit the parent's environment by default on Unix.
- The daemon is the parent of all future sessions, so the env vars propagate transitively.

## Related

- [cct env uses exec, not a shell](cct-env-exec-no-shell-expansion.md) — why `cct env` can't expand `$VAR` or globs
- `src/launch.rs:165` — `exec_with_env()` implementation
