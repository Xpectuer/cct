---
title: "tcp_port_owner_fallback — Red Phase"
brief: "tcp_port_owner_fallback — Red: exit 101"
doc_type: proc
created: 2026-08-01T09:49:51Z
case: "tcp_port_owner_fallback"
phase: red
---
Exit code: 101
Full output: `cargo test tcp_port_owner`（工作树根目录执行；rtk 压缩输出已从 ~/Library/Application Support/rtk/tee/1785577783_cargo_test.log 恢复完整日志，完整输出如下）

```
    Blocking waiting for file lock on artifact directory
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
error[E0425]: cannot find function `tcp_port_owner` in this scope
   --> src/proxy.rs:702:21
    |
702 |         let owner = tcp_port_owner(19191);
    |                     ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `port_conflict_message` in this scope
   --> src/proxy.rs:708:19
    |
708 |         let msg = port_conflict_message(19191);
    |                   ^^^^^^^^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `tcp_port_owner` in this scope
   --> src/proxy.rs:725:21
    |
725 |         let owner = tcp_port_owner(port);
    |                     ^^^^^^^^^^^^^^ not found in this scope

error[E0425]: cannot find function `port_conflict_message` in this scope
   --> src/proxy.rs:730:19
    |
730 |         let msg = port_conflict_message(port);
    |                   ^^^^^^^^^^^^^^^^^^^^^ not found in this scope

For more information about this error, try `rustc --explain E0425`.
error: could not compile `cct` (lib test) due to 4 previous errors
```
