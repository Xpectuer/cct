## Step 5 — Update UI copy for inline edit mode
### Actions Taken
- Added pure UI copy helpers for the form panel title, confirmation prompt, and normal-mode footer so add/edit copy is consistent and testable.
- Updated the add-form panel title to switch between `Add Profile` and `Edit Profile`.
- Changed confirmation copy to be mode-specific and replaced footer `[e] Edit config` text with `[e] Edit`.
- Added a focused `ui_form_title_and_confirmation_reflect_edit_mode` test and updated footer assertions to enforce the new copy.

### Verify Result
- `cargo test --lib ui_ -- --test-threads=1` exited 0.
- `rg -n "Edit config" src/ui.rs` returned no matches.
