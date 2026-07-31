---
title: Step 02
doc_type: proc
brief: Step 02
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 2 — Env var generation tests

### Red

Assessed what Case 1 already delivered:
- `append_profile_generates_env_section` already covers test 1 (full env vars with base_url, api_key, model — asserts all 9 env vars present, round-trips through `load_profiles()`). Skipped writing a duplicate.
- All existing `NewProfile` constructions already have `base_url: None, api_key: None` from Case 1.

Wrote 2 new tests in `src/config.rs`:

1. **`append_profile_base_url_only`** — Creates a profile with only `base_url` set (no api_key, no model). Asserts:
   - `[profiles.env]` section exists in raw TOML output
   - `ANTHROPIC_BASE_URL` present with correct value
   - `ANTHROPIC_MODEL` and `API_TIMEOUT_MS` are NOT present (model was None)
   - Round-trip through `load_profiles()` confirms env map has base_url but not model

2. **`append_minimal_no_env_section`** — Creates a profile with all optional fields `None`. Asserts:
   - The appended block does NOT contain `[profiles.env]`
   - No `ANTHROPIC_BASE_URL`, `ANTHROPIC_API_KEY`, or `ANTHROPIC_MODEL` in output
   - Round-trip confirms `env` is `None`

Both tests passed immediately because the implementation already exists from Case 1. This is expected — in TDD when the implementation predates the tests, the Red phase is that the tests did not exist before. The tests now serve as regression guards for edge cases.

### Green

All 33 tests pass (22 unit + 2 main + 5 integration + 4 live):
```
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

No implementation changes needed.

### Refactor

Reviewed test code for clarity and DRYness:
- `append_minimal_no_env_section` vs `append_minimal_profile`: They test different aspects (raw TOML absence of env section vs round-trip field values). Both are warranted.
- The tempdir + CCT_CONFIG setup pattern is repeated across tests. A helper could be extracted but would touch Case 1 tests (out of scope). Left as-is.
- `cargo clippy` passes clean — no warnings.

### Verify Result

**STATUS: SUCCESS**

- Tests added: 2 (`append_profile_base_url_only`, `append_minimal_no_env_section`)
- Tests skipped: 1 (`append_profile_with_env_vars` — already covered by Case 1's `append_profile_generates_env_section`)
- Total tests: 33 (was 31)
- All pass, clippy clean

### Files Changed
- `src/config.rs` — added 2 new test functions (lines 323-431)
