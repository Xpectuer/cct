---
title: "External Tool Config Schema Must Be Verified Against Working Examples"
doc_type: lesson
brief: "When writing config or auth files for external CLI tools, verify the exact schema from a working example — never guess key names or field presence"
confidence: verified
created: 2026-03-25
updated: 2026-03-25
revision: 1
---

# Lesson: External Tool Config Schema Must Be Verified Against Working Examples

## Context

During the `codex` auth sync feature (`write_codex_auth`), the task was to write an
`auth.json` file to `$CODEX_HOME/` so that codex CLI could pick up the API key at
launch time, without requiring the user to configure it separately.

The initial schema was guessed based on a plausible-looking convention.

## The Bug

The first implementation produced:

```json
{"openai_api_key":"sk-xxx"}
```

This schema was wrong on two counts:

1. **Wrong key casing**: The field was `openai_api_key` (all lowercase), but codex
   expects `OPENAI_API_KEY` (uppercase, matching the env var convention).
2. **Missing required field**: The `auth_mode` field was absent entirely. Codex uses
   `auth_mode` to determine how credentials are sourced; without it, the file is likely
   ignored or misinterpreted.

The correct schema is:

```json
{
  "auth_mode": "apikey",
  "OPENAI_API_KEY": "sk-xxx"
}
```

## Root Cause

The schema was inferred by analogy (snake_case JSON key matching the concept of an API
key) rather than verified against an actual working `auth.json` produced by codex itself
or its documentation.

External CLI tools often have their own internal conventions that do not follow generic
patterns. Config and auth file formats are part of the tool's internal contract, not a
public API — they can use any casing, any field names, and any required fields without
following external conventions.

## The Fix

Changed `write_codex_auth` to emit the verified schema:

```rust
let json = format!(
    "{{\n  \"auth_mode\": \"apikey\",\n  \"OPENAI_API_KEY\": \"{api_key}\"\n}}\n"
);
```

The fix also added a regression test asserting the exact field names:

```rust
assert!(content.contains("\"OPENAI_API_KEY\": \"sk-new456\""));
```

## Rule Derived

> When generating a config or auth file consumed by an external tool, always find or
> produce a real working example of that file first. Verify field names (including
> casing), required fields, and value formats. Do not guess from the concept name.

## Verification Methods (in priority order)

1. **Run the tool yourself**: Let the tool write the file in its own flow, then `cat` it.
2. **Check the tool's source**: Search for the struct or schema that deserializes the file.
3. **Check official documentation**: Only trust docs that include a full worked example.
4. **Search for real user configs**: Community examples in issues or repos can confirm
   the schema, but verify across multiple sources.

Analogy from variable naming (e.g., env var `OPENAI_API_KEY` → JSON key
`openai_api_key`) is **not a verification method**.

## Symptoms to Watch For

- A file is written but the external tool silently ignores it.
- The tool falls back to interactive auth or fails with a credential error even though
  the file exists.
- Field names in the written file differ in casing from what the tool's own logs or
  error messages report.

## Related

- `docs/procs/tdd-codex-auth-sync-20260325140224/` — the TDD session where this was found and fixed
- `src/launch.rs` — `write_codex_auth()` and its tests
