---
title: Step 09
doc_type: proc
brief: Step 09
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 9 — toggle_skip_permissions tests

### Red

Added 3 test cases (`toggle_skip_permissions_insert`, `toggle_skip_permissions_flip`, `toggle_skip_permissions_not_found`) before the `append_minimal_profile` test in `src/config.rs`. Ran `cargo test toggle_skip_permissions` — compilation failed with `E0425: cannot find function toggle_skip_permissions in this scope` (4 occurrences across the 3 tests). Red confirmed.

### Green

Added `toggle_skip_permissions()` function before the `#[cfg(test)]` block in `src/config.rs`. The function uses `toml_edit::DocumentMut` for surgical edits that preserve comments and formatting. It:

1. Reads the config file
2. Parses it as a `DocumentMut`
3. Finds the named profile in the `[[profiles]]` array
4. Sets `skip_permissions` to the new value
5. Writes back, preserving all comments and formatting

Ran `cargo test --lib toggle_skip_permissions` — all 3 tests passed. Green confirmed.

### Refactor

Reviewed the implementation. The function is concise and follows the same patterns as existing code (uses `config_path()`, `anyhow::Context`, `fs::read_to_string`/`fs::write`). No refactoring needed.

### Verify Result

Full test suite: **41 tests passed, 0 failed** across all test targets (lib, main, integration, live, doc-tests).

```
cargo test toggle_skip_permissions:
  toggle_skip_permissions_insert ... ok
  toggle_skip_permissions_flip ... ok
  toggle_skip_permissions_not_found ... ok
```

**Result: SUCCESS**
