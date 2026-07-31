---
title: Step 01
doc_type: proc
brief: Step 01
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 1 -- Backend enum, Profile fields, validation in config.rs

### Actions Taken

1. **Backend enum added** (`src/config.rs`):
   - `Backend` enum with `Claude` (default) and `Codex` variants, `#[serde(rename_all = "lowercase")]`.
   - Derives: `Debug, Default, Deserialize, Clone, PartialEq`.

2. **Profile struct extended** with 3 new fields:
   - `#[serde(default)] pub backend: Backend`
   - `pub base_url: Option<String>`
   - `pub full_auto: Option<bool>`

3. **NewProfile struct extended** with:
   - `pub backend: Backend`
   - `pub full_auto: Option<bool>`

4. **`validate_profiles()` function added**:
   - Rejects codex + skip_permissions combo.
   - Rejects claude + full_auto combo.
   - Called from `load_profiles()`.

5. **`append_profile()` updated**:
   - Writes `backend` field when not Claude.
   - Writes `base_url` as profile-level field.
   - Writes `full_auto` when present.
   - Claude backend: existing ANTHROPIC_* env var generation (unchanged).
   - Codex backend: generates only `OPENAI_API_KEY` in env section.

6. **All dependent files updated** (Profile/NewProfile construction sites):
   - `src/launch.rs` -- `profile()` helper in tests
   - `src/ui.rs` -- two Profile literals in tests
   - `src/cli.rs` -- NewProfile construction in `run_add_with()`
   - `src/main.rs` -- NewProfile construction in TUI add-form handler
   - `tests/integration.rs` -- two Profile literals

### 5 TDD Test Cases

| # | Test Name | Status |
|---|-----------|--------|
| 1 | `backend_enum_deserialization` | PASS |
| 2 | `profile_with_base_url_roundtrips` | PASS |
| 3 | `validate_profiles_rejects_codex_skip_permissions` | PASS |
| 4 | `validate_profiles_rejects_claude_full_auto` | PASS |
| 5 | `append_codex_profile_generates_openai_env` | PASS |

### Verify Result

```
cargo test: 49 tests passed (38 lib + 2 main + 5 integration + 4 live), 0 failed
cargo clippy: clean (no warnings)
```

**STATUS: SUCCESS**
