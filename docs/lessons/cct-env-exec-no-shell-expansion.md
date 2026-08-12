---
title: "cct env preserves argument boundaries — request shell syntax explicitly"
doc_type: lesson
brief: "cct env shell-quotes each argument before bash -c, so shell operators and expansions passed as literal arguments stay literal; use sh -c explicitly when needed"
confidence: verified
created: 2026-05-12
updated: 2026-08-12
revision: 2
---

# Lesson: `cct env` Preserves Argument Boundaries — Request Shell Syntax Explicitly

## Context

`cct env <profile> -- <cmd> [args...]` sets the profile's environment variables, joins the supplied argv with `shell_words::join`, and then exec-replaces the process with `bash -c <joined-command>`.

The quoting step is important: each CLI argument remains one shell word. Text such as `$VAR`, `*`, or `|` that reaches `cct` as part of an argument is quoted before the inner Bash parses it, so it is not automatically reinterpreted as shell syntax.

## The Problem

Two related pitfalls:

1. **Quoting the entire command as one argument**: `cct env ccr -- 'echo $VAR'` passes one argv element. `shell_words::join` preserves it as one word, so the inner Bash looks for a command literally named `echo $VAR`.

2. **Literal arguments stay literal**: `cct env ccr -- echo '$VAR'` passes `$VAR` as data. The join step quotes it, so the inner Bash invokes `echo` with the literal string `$VAR`.

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

## Current Implementation

`run_env` in `src/main.rs` calls `shell_words::join` to turn the parsed command arguments into a safely quoted shell command. `exec_with_env` in `src/launch.rs` then runs `bash -c` and exec-replaces the current process. This supports explicit nested shell commands while preventing ordinary argv values from accidentally becoming operators or expansions.
