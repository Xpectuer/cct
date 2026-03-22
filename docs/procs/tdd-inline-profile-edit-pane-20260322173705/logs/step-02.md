## Step 2 — Add config-layer update support for existing profiles
### Actions Taken
- Verified `update_profile(original_name, updated)` is implemented in `src/config.rs` using `toml_edit`.
- Confirmed focused preservation and rename tests exist for extra args, unknown env keys, in-place rename, and missing-profile errors.
- Re-ran the Step 2 verification target after the local tree settled.

### Verify Result
- `cargo test update_profile -- --test-threads=1` exited 0.
