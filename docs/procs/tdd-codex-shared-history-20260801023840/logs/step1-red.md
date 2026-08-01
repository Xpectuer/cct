---
title: "Step 1 — Red Phase"
brief: "Step 1 — Red: exit 101"
doc_type: proc
created: 2026-08-01T06:02:01Z
case: "resolve_codex_layout_returns_shared_and_overlay_paths / resolve_codex_layout_keeps_profile_name_in_overlay_only"
phase: red
---
Exit code: 101

Test command: `cargo test resolve_codex_layout`

Full output:
```
    Blocking waiting for file lock on artifact directory
   Compiling cct v0.1.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-codex-shared-history-20260801023840-20260801024507)
error[E0425]: cannot find function `resolve_codex_layout` in this scope
   --> src/launch.rs:464:22
    |
464 |         let layout = resolve_codex_layout(name);
    |                      ^^^^^^^^^^^^^^^^^^^^ not found in this scope

error[E0282]: type annotations needed
   --> src/launch.rs:480:51
    |
480 |             !layout.shared_home.components().any(|c| c.as_os_str() == name_segment),
    |                                                   ^  - type must be known at this point
    |
help: consider giving this closure parameter an explicit type
    |
480 |             !layout.shared_home.components().any(|c: /* Type */| c.as_os_str() == name_segment),
    |                                                    ++++++++++++

error[E0425]: cannot find function `resolve_codex_layout` in this scope
   --> src/launch.rs:491:22
    |
491 |         let layout = resolve_codex_layout(name);
    |                      ^^^^^^^^^^^^^^^^^^^^ not found in this scope

error[E0282]: type annotations needed
   --> src/launch.rs:500:51
    |
500 |             !layout.shared_home.components().any(|c| c.as_os_str() == name_segment),
    |                                                   ^  - type must be known at this point
    |
help: consider giving this closure parameter an explicit type
    |
500 |             !layout.shared_home.components().any(|c: /* Type */| c.as_os_str() == name_segment),
    |                                                    ++++++++++++

error[E0282]: type annotations needed
   --> src/launch.rs:505:50
    |
505 |             layout.legacy_home.components().any(|c| c.as_os_str() == name_segment),
    |                                                  ^  - type must be known at this point
    |
help: consider giving this closure parameter an explicit type
    |
505 |             layout.legacy_home.components().any(|c: /* Type */| c.as_os_str() == name_segment),
    |                                                   ++++++++++++

Some errors have detailed explanations: E0282, E0425.
For more information about an error, try `rustc --explain E0282`.
error: could not compile `cct` (lib test) due to 5 previous errors
```

Summary: Red phase confirmed — the two new tests (`resolve_codex_layout_returns_shared_and_overlay_paths` and `resolve_codex_layout_keeps_profile_name_in_overlay_only`) fail to compile with `E0425: cannot find function 'resolve_codex_layout'` because `CodexLayout` / `resolve_codex_layout` do not exist yet. The three `E0282` type-annotation errors are cascading from the missing function (the `layout` binding has no type to infer `.components()` from) and will resolve once the pure function is added in the Green phase. Assertions intentionally use relative path segments (`file_name()`, `components()`) instead of hardcoded absolute paths so they are portable across `dirs::config_dir()` environments. Tests fail as required — Green phase not entered.
