| Step | Status | Notes |
|------|--------|-------|
| Step 1 — Extend `FormState` for edit metadata and prefill helpers | ✅ | Added edit metadata, profile prefilling, and passing focused app tests. |
| Step 2 — Add config-layer update support for existing profiles | ✅ | Verified in-place update support and preservation tests in `src/config.rs`. |
| Step 3 — Replace `[e]` external editor path with inline edit entry | ✅ | `[e]` now enters a prefilled inline edit form in `src/main.rs`. |
| Step 4 — Split add-mode and edit-mode save behavior | ✅ | Edit-mode save validates renames, persists updates, and reselects the saved profile. |
| Step 5 — Update UI copy for inline edit mode | ✅ | UI titles, confirmation copy, footer text, and empty-state wording reflect inline edit behavior. |
| Step 6 — Update README to match the new interaction | ✅ | README documents inline edit instead of `$EDITOR`/hot-reload. |
| Step 7 — Add focused tests for preservation and rename behavior | ✅ | All focused app/config/ui/main tests for the feature are present and passing. |
| Step 8 — Final verification | ✅ | `cargo test -- --test-threads=1` and `cargo clippy` both passed. |
