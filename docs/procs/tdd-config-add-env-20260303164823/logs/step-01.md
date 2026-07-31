---
title: Step 01
doc_type: proc
brief: Step 01
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 1 — NewProfile struct extension

### Red

Wrote test `append_profile_generates_env_section` in `src/config.rs` that constructs
`NewProfile` with new `base_url` and `api_key` fields and asserts the output TOML
contains all expected env vars (`ANTHROPIC_BASE_URL`, `ANTHROPIC_API_KEY`,
`ANTHROPIC_MODEL`, `ANTHROPIC_SMALL_FAST_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`,
`ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_HAIKU_MODEL`, `API_TIMEOUT_MS`,
`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`). Also verifies round-trip through
`load_profiles()` parsing.

`cargo test` result: **FAIL** — compilation error:
```
error[E0560]: struct `config::NewProfile` has no field named `base_url`
error[E0560]: struct `config::NewProfile` has no field named `api_key`
```

### Green

1. Added `base_url: Option<String>` and `api_key: Option<String>` to `NewProfile` struct.
2. Rewrote `append_profile()` to generate `[profiles.env]` section when any of
   `base_url`, `api_key`, or `model` are provided:
   - `ANTHROPIC_BASE_URL` from `base_url`
   - `ANTHROPIC_API_KEY` from `api_key`
   - When `model` provided: `ANTHROPIC_MODEL`, `ANTHROPIC_SMALL_FAST_MODEL`,
     `ANTHROPIC_DEFAULT_SONNET_MODEL`, `ANTHROPIC_DEFAULT_OPUS_MODEL`,
     `ANTHROPIC_DEFAULT_HAIKU_MODEL`, `API_TIMEOUT_MS = "600000"`,
     `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"`
3. Updated all `NewProfile` construction sites to add `base_url: None, api_key: None`:
   - `src/config.rs` tests: `append_profile_roundtrips`, `append_preserves_existing`, `append_minimal_profile`
   - `src/cli.rs` line 67: `NewProfile { name, description, base_url: None, api_key: None, model }`
   - `src/main.rs` line 112: `config::NewProfile { name, description, base_url: None, api_key: None, model }`

`cargo test` result: **PASS** — 31/31 tests passed (20 lib + 2 main + 5 integration + 4 live).

### Refactor

Extracted `non_empty()` helper function to eliminate repetitive `opt.as_ref().is_some_and(|s| !s.is_empty())`
and `if let Some(x) = &field { if !x.is_empty() { ... } }` patterns. This flattened the
nested conditionals in `append_profile()` and improved readability.

`cargo test` result: **PASS** — 31/31 tests passed.
`cargo clippy` result: **CLEAN** — no warnings.

### Verify Result

**STATUS: SUCCESS**

All phases completed cleanly:
- RED: Test failed as expected (struct fields missing)
- GREEN: All 31 tests pass with new fields and env generation logic
- REFACTOR: Extracted `non_empty()` helper, all tests still pass, clippy clean

Files modified:
- `src/config.rs` — `NewProfile` struct extended, `append_profile()` rewritten, `non_empty()` helper added, new test added
- `src/cli.rs` — `NewProfile` construction updated with new fields
- `src/main.rs` — `NewProfile` construction updated with new fields
