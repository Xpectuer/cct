## Step 1 — Extend FormState for edit metadata and prefill helpers
### Actions Taken
- Added `is_edit` and `original_name` to `FormState`.
- Added `FormState::new_for_backend()` and `FormState::from_profile()` to prefill Claude and Codex edit forms from an existing profile.
- Added focused tests for Claude and Codex prefill behavior and tightened the existing mode-transition assertion for the new edit metadata defaults.

### Verify Result
- `cargo test from_profile -- --test-threads=1` exited 0.
