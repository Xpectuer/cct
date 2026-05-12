---
title: "cct env uses exec, not a shell — no variable expansion"
doc_type: lesson
brief: "cct env exec-replaces directly, so shell features ($VAR expansion, globs, pipes) don't work; wrap in sh -c when needed"
confidence: verified
created: 2026-05-12
updated: 2026-05-12
revision: 1
---

# Lesson: cct env uses exec, not a shell — no variable expansion

## Context

`cct env <profile> -- <cmd> [args...]` sets the profile's environment variables and then `exec`-replaces the current process with `<cmd>` directly. There is no intermediate shell.

## The Problem

Two related pitfalls:

1. **Quoting the entire command as one argument**: `cct env ccr -- 'echo $VAR'` passes the whole string `echo $VAR` as the command name. `command_exists` then runs `which "echo $VAR"` — looking for a binary literally named `echo $VAR` — and fails with "Command not found".

2. **No shell expansion**: Even with correct argument splitting, `echo $VAR` prints the literal string `$VAR`. `echo` itself does not expand variables — that is a shell feature. Since `exec_with_env` calls `Command::new(cmd).args(args).exec()`, no shell is ever involved.

## The Fix

Always use `sh -c` when you need shell expansion:

```bash
# Wrong — treated as a single command name
cct env ccr -- 'echo $ANTHROPIC_API_KEY'

# Also wrong — echo doesn't expand variables
cct env ccr -- echo '$ANTHROPIC_API_KEY'

# Correct — sh expands the variable
cct env ccr -- sh -c 'echo $ANTHROPIC_API_KEY'
```

## Root Cause in Code

`exec_with_env` in `src/launch.rs:163` uses `std::process::Command` with `.exec()`, which directly invokes the binary via `execve(2)`. No shell parsing, no variable expansion, no globbing.

`command_exists` in `src/launch.rs:151` runs `which <cmd>` — it does not split `<cmd>` on whitespace, so multi-word strings fail.
