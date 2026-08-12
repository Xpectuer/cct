---
title: "Form Field Index Must Have a Single Source of Truth"
doc_type: lesson
brief: "When UI labels and data mapping share a field-index contract, any divergence silently corrupts user input"
confidence: verified
created: 2026-03-15
updated: 2026-08-12
revision: 2
---

# Lesson: Form Field Index Must Have a Single Source of Truth

## Context

During the codex backend implementation, the TUI add-form was extended to support two different field layouts:

- **Claude**: `[0]=Name, [1]=Description, [2]=Base URL, [3]=API Key, [4]=Pro Model, [5]=Fast Model`
- **Codex**: `[0]=Name, [1]=Base URL, [2]=API Key, [3]=Model, [4]=Approval, [5]=unused`

Both layouts share the same `[String; 6]` buffer in `FormState`. The sixth slot lets Claude expose a separate fast model while Codex leaves that slot unused.

## The Bug

`build_form_lines()` in `ui.rs` (responsible for rendering field labels) was hardcoded to Claude labels for both the **edit view** and the **confirmation view**. When a user created a Codex profile:

1. The form showed "Description" as the label for field index 1.
2. The user entered the Base URL into what they saw as "Description".
3. `main.rs` (before the fix) read `fields[1]` as `description` (Claude convention).
4. The actual Base URL the user typed was silently lost; `base_url` in the resulting `NewProfile` was `None`.

This was a **silent data loss bug** — no error, no warning, wrong output.

## The Fix

Two changes were made together:

### 1. `field_labels(backend: &Backend) -> [&'static str; 6]` in app.rs

A free function that returns the correct label array for the active backend. `build_form_lines()` calls `field_labels(&form.backend)` dynamically instead of hard-coding `FIELD_LABELS`.

### 2. `FormState::to_new_profile(&self) -> NewProfile` in app.rs

A method that reads `self.fields` according to `self.backend`'s index convention and constructs the `NewProfile`. This is the **single source of truth** for the field-index-to-semantic mapping.

Before this fix, `main.rs` contained its own inline field-index reads (e.g., `fields[1]` for description, `fields[2]` for base_url), which were a separate copy of the same contract and could drift from what `build_form_lines()` rendered.

After the fix, `main.rs` calls `form.to_new_profile()` — no field indices appear outside `app.rs`.

## Rule Derived

> When a `[T; N]` array is used as a polymorphic buffer where the semantic meaning of each index depends on a runtime discriminant (like `Backend`), all reads from that array must go through a single function that knows the discriminant. Duplicating the index convention at call sites creates a silent contract that is impossible to enforce statically.

## Symptoms to Watch For

- UI label set and data-extraction code live in different files/functions.
- A form works correctly for one variant but silently misbehaves for another.
- No compile-time error when adding a new variant — the new variant just inherits the old one's index mapping.

## Test Pattern

The regression tests added in Step 5 verify this by asserting:

```rust
// field_labels and to_new_profile must agree on field[1] for Codex
let labels = field_labels(&Backend::Codex);
assert_eq!(labels[1], "Base URL");

let mut form = FormState::new();
form.backend = Backend::Codex;
form.fields[1] = "https://api.example.com".into();
let np = form.to_new_profile();
assert_eq!(np.base_url.as_deref(), Some("https://api.example.com"));
```

This pattern should be replicated whenever a new backend or form variant is added.
