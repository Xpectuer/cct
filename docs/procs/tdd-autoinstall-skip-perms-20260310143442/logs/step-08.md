---
title: Step 08
doc_type: proc
brief: Step 08
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Step 8 — check_claude_installed tests

### Red

Added two test functions to `src/launch.rs` `mod tests` block, before `build_args_empty`:
- `check_claude_installed_found` — sets `CCT_CLAUDE_BIN=true` and asserts `check_claude_installed()` returns true
- `check_claude_installed_not_found` — sets `CCT_CLAUDE_BIN=nonexistent-binary-xyz-12345` and asserts false

Result: **Compilation failed** as expected — `cannot find function check_claude_installed in module super` (2 errors).

### Green

Added production code to `src/launch.rs` before the `open_editor` function:
- `pub fn check_claude_installed() -> bool` — uses `which` to check if `claude` (or `CCT_CLAUDE_BIN` override) exists in PATH
- `pub fn prompt_install() -> Result<()>` — interactive install prompt using the official installer script, with fallback check for `~/.local/bin/claude`

Result: **Both tests pass.** Full test suite (30 tests) also passes.

### Refactor

Ran `cargo clippy` — no warnings. Code is clean, no refactoring needed.

### Verify Result

```
cargo test check_claude_installed

running 2 tests
test launch::tests::check_claude_installed_found ... ok
test launch::tests::check_claude_installed_not_found ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out
```

**STATUS: SUCCESS**
