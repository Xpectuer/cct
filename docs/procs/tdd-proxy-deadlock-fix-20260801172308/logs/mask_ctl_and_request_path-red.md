---
title: "mask_ctl_and_request_path — Red Phase"
brief: "mask_ctl_and_request_path — Red: exit 101"
doc_type: proc
created: 2026-08-01T09:58:59Z
case: "mask_ctl_and_request_path"
phase: red
---
Exit code: 101
Full output: `cargo test mask_`（工作树根目录执行；rtk 压缩输出已从 ~/Library/Application Support/rtk/tee/1785578333_cargo_test.log 恢复完整日志，完整输出如下）

```
    Blocking waiting for file lock on artifact directory
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
error[E0425]: cannot find function `mask_ctl_line` in this scope
   --> src/proxy.rs:774:22
    |
774 |         let masked = mask_ctl_line(line, Some("sk-abc123"));
    |                      ^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `mask_ctl_line` in this scope
   --> src/proxy.rs:790:22
    |
790 |         let masked = mask_ctl_line(line, Some("custom-token-xyz"));
    |                      ^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `mask_ctl_line` in this scope
   --> src/proxy.rs:805:20
    |
805 |         assert_eq!(mask_ctl_line(line, None), line);
    |                    ^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `mask_request_path` in this scope
   --> src/proxy.rs:812:22
    |
812 |         let masked = mask_request_path(path);
    |                      ^^^^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `mask_request_path` in this scope
   --> src/proxy.rs:827:22
    |
827 |         let masked = mask_request_path(path);
    |                      ^^^^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `mask_request_path` in this scope
   --> src/proxy.rs:842:20
    |
842 |         assert_eq!(mask_request_path(path), path);
    |                    ^^^^^^^^^^^^^^^^^ not found in this scope

For more information about this error, try `rustc --explain E0425`.
error: could not compile `cct` (lib test) due to 6 previous errors
```
