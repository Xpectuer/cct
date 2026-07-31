---
title: Case 03
doc_type: proc
brief: Case 03
confidence: speculative
created: 2026-06-30
updated: 2026-06-30
revision: 1
---
## Case 3 — build_args_continue_with_flags

### Actions Taken

1. **RED**: Added `build_args_continue_with_flags` test to `src/launch.rs` test module after `build_args_continue_only`. The test creates a profile with model="opus", skip_permissions=true, extra_args=["--verbose"] and asserts `build_args(&p, true)` produces `["--continue", "--model", "opus", "--dangerously-skip-permissions", "--verbose"]`.
2. **GREEN**: Test passed immediately — the `build_args` implementation from Cases 1-2 already handles the combined case correctly. `--continue` is emitted first when `with_continue=true`, followed by model, skip_permissions, and extra_args in order.
3. **REFACTOR**: No refactoring needed. The logic is clean and the argument ordering is consistent.

### Verify Result

- `cargo test build_args_continue_with_flags` — PASSED
- `cargo test` (full suite) — 44 tests passed, 0 failed

**Result: SUCCESS**
